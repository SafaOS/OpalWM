//! Opal ABI Macros to generate messages and such

use quote::quote;
use syn::DeriveInput;

struct MessageVariant {
    name: syn::Ident,
    types: Vec<(String, syn::Type)>,
    op_code: u16,
    is_named: bool,
}

struct MessageEnum {
    name: syn::Ident,
    variants: Vec<MessageVariant>,
    /// Meaning the OpCode is repeated at the end, wrapping the message.
    wrapped: bool,
    generics: syn::Generics,
}

fn query_message_enum(
    name: syn::Ident,
    generics: syn::Generics,
    wrapped: bool,
    en: syn::DataEnum,
) -> MessageEnum {
    let mut variants: Vec<MessageVariant> = Vec::with_capacity(en.variants.len());
    let mut last_op_code = 0u16;

    for variant in en.variants {
        let mut is_named = false;
        let types = variant
            .fields
            .into_iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.ident
                        .map(|id| {
                            is_named = true;
                            id.to_string()
                        })
                        .unwrap_or(format!("field_{i}")),
                    f.ty,
                )
            })
            .collect::<Vec<_>>();
        assert!(!types.is_empty(), "Empty enum variant not allowed");

        let op_code = variant
            .discriminant
            .map(|(_, dis)| match dis {
                syn::Expr::Lit(liter) => match liter.lit {
                    syn::Lit::Int(l) => l
                        .base10_parse::<u16>()
                        .expect("Failed to parse enum discriminant into a u16"),
                    _ => panic!("Unsupported enum discriminant"),
                },
                _ => panic!("Unsupported enum discrimination"),
            })
            .unwrap_or(last_op_code + 1);

        assert!(
            variants.iter().all(|v| v.op_code != op_code),
            "Duplicate Op Code for message"
        );
        last_op_code = op_code;

        variants.push(MessageVariant {
            name: variant.ident,
            types,
            op_code,
            is_named,
        });
    }

    assert!(!variants.is_empty(), "Variantless enums are not allowed");
    MessageEnum {
        name,
        variants,
        wrapped,
        generics,
    }
}

fn derive_message_enum(queried_info: MessageEnum) -> proc_macro::TokenStream {
    let name = queried_info.name;
    let is_wrapped = queried_info.wrapped;
    let mut encode_arms = Vec::with_capacity(queried_info.variants.len());
    let mut decode_arms = Vec::with_capacity(queried_info.variants.len());

    for variant in queried_info.variants {
        let encode_names = variant
            .types
            .iter()
            .map(|(name, _)| quote::format_ident!("{name}"));

        let the_thing = if variant.is_named {
            quote::quote! {{#(#encode_names),*}}
        } else {
            quote::quote! {(#(#encode_names),*)}
        };

        let decode_fields = variant.types.iter().map(|(name, ty)| {
            let name = quote::format_ident!("{}", name);
            quote::quote! {
                let (#name, size) = <#ty>::decode_from_buf(buf.get(data_read..).ok_or(DecodeError::BufferTooSmall)?)?;
                data_read += size;
            }
        });

        let decode_fields = quote::quote! { #(#decode_fields)* };

        let op_code = variant.op_code;
        let decode_op_code_end = is_wrapped.then(|| {
            quote::quote! {
                let (op_code, size) = u16::decode_from_buf(buf.get(data_read..).ok_or(DecodeError::BufferTooSmall)?)?;
                data_read += size;

                if op_code != #op_code {
                    return Err(DecodeError::UnexpectedEnd.into());
                }
            }
        });
        let decode_op_code_end = decode_op_code_end.iter();

        let encode_fields = variant.types.into_iter().map(|(name, _)| {
            let name = quote::format_ident!("{}", name);
            quote::quote! {
                {
                    let value = #name;
                    encoded += value.encode_into_buf(buf.get_mut(encoded..).ok_or(())?).map_err(|_| ())?;
                }
            }
        });

        let var_name = quote::format_ident!("{}", variant.name);
        let encode_op_code = quote::quote! {
            encoded += (#op_code).encode_into_buf(buf.get_mut(encoded..).ok_or(())?).map_err(|_| ())?;
        };

        let encode_op_code_end = is_wrapped.then(|| &encode_op_code);
        let encode_op_code_end = encode_op_code_end.iter();

        encode_arms.push(quote::quote! {
            Self::#var_name #the_thing => {
                #encode_op_code
                #(#encode_fields)*
                #(#encode_op_code_end)*
            }
        });

        decode_arms.push(quote::quote! {
            #op_code => {
                #decode_fields
                #(#decode_op_code_end)*

                Self::#var_name #the_thing
            }
        });
    }

    let generics = queried_info.generics;
    quote::quote! {
        impl #generics #name #generics {
            /// FIXME: not exactly accurate but that was the easiest way to do it, could break tho.
            const MAX_ENCODE_SIZE: usize = size_of::<Self>() + 128;

            #[inline]
            /// Encode the message to the given buffer.
            pub fn encode_into_buf(&self, buf: &mut [u8]) -> Result<usize, ()> {
                use crate::encoding::*;
                let mut encoded = 0;

                match self {
                    #(#encode_arms)*
                }
                Ok(encoded)
            }

            #[inline]
            /// Encode the message to the given writer.
            pub fn encode_into<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
                let mut buf = [0u8; Self::MAX_ENCODE_SIZE];
                let encoded = self.encode_into_buf(&mut buf).expect("Buffer should be large enough");
                writer.write_all(&buf[..encoded])?;
                Ok(encoded)
            }

            #[inline]
            pub fn decode_from_buf(buf: &[u8]) -> Result<(Self, usize), crate::encoding::DecodeError> {
                use crate::encoding::*;
                let mut data_read = 0;

                let (op_code, size) = u16::decode_from_buf(buf)?;
                data_read += size;

                let decoded = match op_code {
                    #(#decode_arms)*
                    _ => return Err(crate::encoding::DecodeError::InvalidOpCode(op_code)),
                };

                Ok((decoded, data_read))
            }

            #[inline]
            /// Decode the message from the given reader.
            pub fn decode_from<R: std::io::Read>(reader: &mut R) -> Result<(Self, usize), crate::encoding::DecodeErrorOrIo> {
                let mut buf = [0u8; Self::MAX_ENCODE_SIZE];
                let size = reader.read(&mut buf)?;

                let (decoded, size) = Self::decode_from_buf(&buf[..size])?;
                Ok((decoded, size))
            }
        }
    }
    .into()
}

struct MessageParam {
    name: syn::Ident,
    r#type: syn::Type,
    /// If is optional we have a default value
    default_value: Option<Box<syn::Expr>>,
    constructor_function: Option<Box<(syn::Expr, syn::Type)>>,
    into_opt_f: Option<Box<syn::Expr>>,
}

struct MessageStruct {
    name: syn::Ident,
    params: Vec<MessageParam>,
    generics: syn::Generics,
}

fn query_message_struct(
    name: syn::Ident,
    generics: syn::Generics,
    st: syn::DataStruct,
) -> MessageStruct {
    let mut params = Vec::with_capacity(st.fields.len());

    for field in st.fields {
        let f_name = field.ident.expect("Identifierless field");
        let f_type = field.ty;
        let mut f_default: Option<Box<syn::Expr>> = None;
        let mut f_constructor_function: Option<Box<(syn::Expr, syn::Type)>> = None;
        let mut f_into_opt: Option<Box<syn::Expr>> = None;

        match f_type {
            syn::Type::Path(ref path_type) => {
                let segments = &path_type.path.segments;
                match segments.last() {
                    Some(i) if i.ident == "Option" => {
                        let syn::PathArguments::AngleBracketed(args) = &i.arguments else {
                            continue;
                        };
                        let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() else {
                            continue;
                        };

                        f_default = Some(Box::new(syn::parse_quote! { None }));
                        f_constructor_function = Some(Box::new((
                            syn::parse_quote! { Option::Some },
                            inner_ty.clone(),
                        )));
                        f_into_opt = Some(Box::new(syn::parse_quote! { core::convert::identity }));
                    }
                    None => {}
                    Some(_) => {}
                }
            }
            _ => {}
        }

        params.push(MessageParam {
            name: f_name,
            r#type: f_type,
            default_value: f_default,
            constructor_function: f_constructor_function,
            into_opt_f: f_into_opt,
        });
    }

    MessageStruct {
        name,
        params,
        generics,
    }
}

fn derive_message_struct(mut queried_info: MessageStruct) -> proc_macro::TokenStream {
    let name = &queried_info.name;
    let size_sum = queried_info
        .params
        .is_empty()
        .then(|| quote::quote! {size_of::<u8>()})
        .unwrap_or_else(|| {
            let fields_len = queried_info.params.len();

            let fields_sizes = queried_info.params.iter().map(|p| {
                let r#type = &p.r#type;
                quote::quote! { <#r#type as crate::encoding::HasMaxEncodeSize>::ENCODE_SIZE }
            });

            let params_headers_size = quote::quote! { size_of::<u8>() * #fields_len };
            quote::quote! {
                size_of::<u8>() + (#(#fields_sizes)+*) + (#params_headers_size)
            }
        });

    let constructor_args_part = queried_info
        .params
        .iter()
        .filter(|p| p.default_value.is_none())
        .map(|param| {
            let name = &param.name;
            let r#type = &param.r#type;
            quote::quote! { #name: #r#type }
        });
    let constructor_args_part = quote::quote! { #(#constructor_args_part),* };

    let constructor_values = queried_info.params.iter().map(|param| {
        let default_value = param.default_value.as_ref();
        let name = &param.name;
        let value = default_value
            .map(|v| quote::quote! { #v })
            .unwrap_or_else(|| quote::quote! { #name });
        quote::quote! { #name: #value }
    });

    let constructor_value = quote::quote! {
        Self {
            #(#constructor_values),*
        }
    };

    let constructor = quote::quote! {
        /// Constructs a new instance of the message [`Self`], with default values for optional fields.
        pub const fn new(#constructor_args_part) -> Self {
            #constructor_value
        }
    };

    drop(constructor_value);
    drop(constructor_args_part);

    let encoding_methods = {
        let encode_params_len = {
            let count: u8 = queried_info
                .params
                .len()
                .try_into()
                .expect("Message has way too many params");

            let sub_valueless = queried_info.params.iter().filter_map(|param| {
                let name = &param.name;
                param.into_opt_f.as_ref().map(|f| {
                    quote::quote! {
                        if (#f)(self.#name).is_none() {
                            params_count -= 1;
                        }
                    }
                })
            });
            quote! {{
                let mut params_count = #count;
                #(#sub_valueless)*
                array[offset] = params_count;
                offset += 1;
            }}
        };

        let encode_into_array_each =
            queried_info
                .params
                .iter_mut()
                .enumerate()
                .map(|(index, param)| {
                    let name = &param.name;
                    let into_opt = param.into_opt_f.take();

                    let get_value = if let Some(into_opt) = into_opt {
                        quote::quote! {
                            let Some(real_value) = (#into_opt)(&self.#name) else {
                                break 'blk;
                            };
                            let value = real_value;
                        }
                    } else {
                        quote::quote! {
                            let value = &self.#name;
                        }
                    };

                    quote::quote! {
                        'blk: {
                            #get_value

                            let p_index: u8 = (#index) as u8;
                            array[offset] = p_index;
                            offset += 1;

                            let (_, encode_part) = array.split_at_mut(offset);
                            offset += value.encode_into_buf(encode_part).expect("Failed to encode parameter: MAX_ENCODE_SIZE too small");
                        }
                    }
                });

        quote::quote! {
            pub const MAX_ENCODE_SIZE: usize = #size_sum;

            #[inline]
            /// Encodes the message into an array of bytes.
            pub fn encode_into_array(&self, array: &mut [u8; Self::MAX_ENCODE_SIZE]) -> usize {
                use crate::encoding::*;
                let mut offset = 0;
                #encode_params_len;
                #(#encode_into_array_each);*
                offset
            }

            #[inline(always)]
            /// Encodes the message into a buffer slice.
            pub fn encode_into_buf(&self, buf: &mut [u8]) -> Result<usize, ()> {
                if let Some(array) = buf.first_chunk_mut::<{ Self::MAX_ENCODE_SIZE }>() {
                    Ok(self.encode_into_array(array))
                } else {
                    Err(())
                }
            }

            #[inline]
            /// Encodes the message into a writer.
            pub fn encode_into(&self, writer: &mut impl std::io::Write) -> Result<usize, std::io::Error> {
                let mut array = [0u8; Self::MAX_ENCODE_SIZE];
                let size = self.encode_into_array(&mut array);
                writer.write_all(&array[..size]).map(|_| size)
            }
        }
    };

    let decode_methods = {
        let max_params = queried_info.params.len() as u8;
        let initializers = queried_info.params.iter().map(|param| {
            let name = &param.name;
            let r#type = &param.r#type;
            if let Some(ref default_value) = param.default_value {
                quote::quote! {
                    let mut #name: #r#type = #default_value;
                }
            } else {
                quote::quote! {
                    let mut #name: Option<#r#type> = None;
                }
            }
        });

        let assigners = queried_info.params.iter().map(|param| {
            let name = &param.name;
            let r#type = &param.r#type;

            if let Some((construct, real_type)) = param.constructor_function.as_deref() {
                quote::quote! {
                    if buf.len() <= data_read {
                        return Err(DecodeError::BufferTooSmall.into());
                    }

                    let (v, size) = <#real_type>::decode_from_buf(&buf[data_read..])?;
                    data_read += size;
                    #name = (#construct)(v);
                }
            } else {
                quote::quote! {
                    if buf.len() <= data_read {
                        return Err(DecodeError::BufferTooSmall.into());
                    }

                    let (v, size) = <#r#type>::decode_from_buf(&buf[data_read..])?;
                    data_read += size;
                    #name = Some(v);
                }
            }
        });

        let match_arms = assigners.enumerate().map(|(index, assigner)| {
            let index = index as u8;

            quote::quote! {
                #index => {
                    #assigner
                }
            }
        });

        let decode_body = quote::quote! {
            use crate::encoding::*;
            let mut data_read = 0;
            #(#initializers)*

            if buf.len() <= data_read {
                return Err(DecodeError::BufferTooSmall.into());
            }

            let params_len = buf[data_read];
            data_read += 1;

            if params_len > #max_params {
                return Err(DecodeError::TooManyParams.into());
            }

            for _ in 0..params_len {
                if buf.len() <= data_read {
                    return Err(DecodeError::BufferTooSmall.into());
                }

                let param_index = buf[data_read];
                data_read += 1;

                match param_index {
                    #(#match_arms)*
                    _ => return Err(DecodeError::InvalidParam(param_index).into()),
                }
            }
        };

        let initializers_final = queried_info.params.iter().map(|param| {
            let name = &param.name;
            if param.default_value.is_some() {
                quote::quote! {
                    #name
                }
            } else {
                quote::quote! {
                    #name: #name.ok_or(DecodeError::MissingParam)?
                }
            }
        });

        quote::quote! {
            #[inline]
            /// Decodes a message [`Self`] from the given buffer.
            pub fn decode_from_buf(buf: &[u8]) -> Result<(Self, usize), crate::encoding::DecodeError> {
                #decode_body

                Ok((Self {
                    #(#initializers_final),*
                }, data_read))
            }

            #[inline]
            /// Decodes a message [`Self`] from the given reader.
            pub fn decode_from<R: std::io::Read>(reader: &mut R) -> Result<(Self, usize), crate::encoding::DecodeErrorOrIo> {
                let mut buf = [0u8; Self::MAX_ENCODE_SIZE];
                let size = reader.read(&mut buf)?;
                Self::decode_from_buf(&buf[..size]).map_err(|err| err.into())
            }
        }
    };

    let default_values_methods = queried_info.params.iter_mut().filter_map(|param| {
        let (constructor_function, real_type) = &*param.constructor_function.take()?;
        let name = &param.name;

        let f_name = quote::format_ident!("with_{}", name);
        Some(quote::quote! {
            #[inline(always)]
            /// Constructs a new instance of the message [`Self`] with the specified value for the field `#name`.
            pub const fn #f_name(mut self, value: #real_type) -> Self {
                self.#name = (#constructor_function)(value);
                self
            }
        })
    });

    let generics = queried_info.generics;
    quote::quote! {
        impl #generics #name #generics {
            #constructor
            #(#default_values_methods)*
            #decode_methods
            #encoding_methods
        }
    }
    .into()
}

fn derive_encodeable(input: syn::DeriveInput) -> proc_macro::TokenStream {
    match input.data {
        syn::Data::Enum(en) => {
            let is_wrapped = input.attrs.iter().any(|attr| attr.path.is_ident("wrapped"));
            derive_message_enum(query_message_enum(
                input.ident,
                input.generics,
                is_wrapped,
                en,
            ))
        }
        syn::Data::Struct(st) => {
            derive_message_struct(query_message_struct(input.ident, input.generics, st))
        }
        syn::Data::Union(_) => unreachable!("Unions cannot be messages"),
    }
}

#[proc_macro_derive(EncodeableMessage, attributes(wrapped))]
pub fn size_bytes(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    derive_encodeable(ast)
}
