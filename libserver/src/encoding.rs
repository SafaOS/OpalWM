use std::num::NonZero;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferTooSmall;

/// An error that occurs when decoding a message from raw bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DecodeError {
    #[error("Invalid OpCode: {0:#x}")]
    /// Given message contains an invalid opcode.
    InvalidOpCode(u16),
    #[error("Buffer Too Small")]
    /// Given buffer is too small to possibly decode the message.
    BufferTooSmall,
    #[error("Invalid Data")]
    /// Given buffer contains invalid data, i.e the data is not in the expected format.
    InvalidData,
    #[error("Invalid Parameter {0}")]
    /// Given buffer contains an invalid parameter, if decoding a struct, the parameter index is out of range, if decoding an enum, the variant discriminantion is out of range.
    InvalidParam(u8),
    #[error("Too Many Parameters")]
    /// Given buffer contains too many parameters.
    TooManyParams,
    #[error("Missing Parameter")]
    /// Given buffer is missing a required parameter for a struct.
    MissingParam,
    #[error("Unexpected End")]
    /// Message ends too soon or with unexpected magic or a token.
    UnexpectedEnd,
    #[error("Unexpected Message")]
    /// Message is not of the expected type.
    UnexpectedMessage,
}

/// A [`DecodeError`] or [`std::io::Error`].
#[derive(Debug, Error)]
pub enum DecodeErrorOrIo {
    #[error("Decode error: {0}")]
    /// See [`DecodeError`].
    DecodeError(#[from] DecodeError),
    #[error("I/O error: {0}")]
    /// An I/O error reading from the given reader.
    Io(#[from] std::io::Error),
}

impl From<BufferTooSmall> for DecodeError {
    fn from(_: BufferTooSmall) -> Self {
        DecodeError::BufferTooSmall
    }
}

pub trait MessageParam: Sized {
    /// The total amount of bytes this will take.
    fn encode_size(&self) -> usize;
    fn encode_into<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error>;
    fn decode_from<R: std::io::Read>(reader: &mut R) -> Result<(Self, usize), DecodeErrorOrIo>;
    fn encode_into_buf(&self, mut buf: &mut [u8]) -> Result<usize, BufferTooSmall> {
        self.encode_into(&mut buf).map_err(|_| BufferTooSmall)
    }
    fn decode_from_buf(mut buf: &[u8]) -> Result<(Self, usize), DecodeError> {
        Self::decode_from(&mut buf).map_err(|_| DecodeError::BufferTooSmall)
    }
}

/// The encoded length of the type is at least the size of type.
pub trait HasMaxEncodeSize: Copy {
    const ENCODE_SIZE: usize = size_of::<Self>();
}

impl<T: HasMaxEncodeSize> HasMaxEncodeSize for Option<T> {
    const ENCODE_SIZE: usize = T::ENCODE_SIZE;
}

/// The encoded length of the type is the size of the type.
pub trait EncodeSizeKnown: HasMaxEncodeSize {}

macro_rules! impl_generic {
    (int $($ty: ty)*) => {
        impl_generic!($($ty)*);

        $(
        impl MessageParam for NonZero<$ty> {
                       #[inline(always)]
                       fn encode_size(&self) -> usize {
                           size_of::<NonZero<$ty>>()
                       }

                       #[inline(always)]
                       fn encode_into_buf(&self, buf: &mut [u8]) -> Result<usize, BufferTooSmall> {
                           self.get().encode_into_buf(buf)
                       }

                       #[inline(always)]
                       fn decode_from_buf(buf: &[u8]) -> Result<(Self, usize), DecodeError> {
                           let (value, bytes_read) = <$ty>::decode_from_buf(buf)?;
                           NonZero::new(value).ok_or(DecodeError::InvalidData).map(|v| (v, bytes_read))
                       }

                       #[inline(always)]
                       fn encode_into<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
                           self.get().encode_into(writer)
                       }

                       #[inline(always)]
                       fn decode_from<R: std::io::Read>(reader: &mut R) -> Result<(Self, usize), DecodeErrorOrIo> {
                           let (value, bytes_read) = <$ty>::decode_from(reader)?;
                           NonZero::new(value).ok_or(DecodeError::InvalidData.into()).map(|v| (v, bytes_read))
                       }
        }

        impl HasMaxEncodeSize for NonZero<$ty> {}
        impl EncodeSizeKnown for NonZero<$ty> {}
        )*
    };

    ($($ty: ty)*) => {
        $(
            impl HasMaxEncodeSize for $ty {}
            impl EncodeSizeKnown for $ty {}

            impl MessageParam for $ty {
                #[inline(always)]
                fn encode_size(&self) -> usize {
                    size_of::<Self>()
                }

                #[inline(always)]
                fn encode_into<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
                    writer.write(bytemuck::bytes_of(self))
                }

                #[inline(always)]
                fn decode_from<R: std::io::Read>(reader: &mut R) -> Result<(Self, usize), DecodeErrorOrIo> {
                    let mut item: Self = unsafe { core::mem::zeroed() };
                    // TODO: Replace with read_exact?
                    if reader.read(bytemuck::bytes_of_mut(&mut item))? != size_of::<Self>() {
                        return Err(DecodeErrorOrIo::DecodeError(DecodeError::BufferTooSmall));
                    }

                    Ok((item, size_of::<Self>()))
                }

                #[inline(always)]
                fn encode_into_buf(&self, buf: &mut [u8]) -> Result<usize, BufferTooSmall> {
                    if buf.len() < size_of::<Self>() {
                        return Err(BufferTooSmall);
                    }

                    (&mut buf[..size_of::<Self>()]).copy_from_slice(bytemuck::bytes_of(self));
                    Ok(size_of::<Self>())
                }

                #[inline(always)]
                fn decode_from_buf(buf: &[u8]) -> Result<(Self, usize), DecodeError> {
                    if buf.len() < size_of::<Self>() {
                        return Err(DecodeError::BufferTooSmall);
                    }

                    Ok((
                        bytemuck::pod_read_unaligned(&buf[..size_of::<Self>()]),
                        size_of::<Self>(),
                    ))
                }
            }
        )*
    };

}
impl_generic!(int u8 u16 u32 u64 i8 i16 i32 i64 usize isize);

#[macro_export]
macro_rules! impl_inheritly {
    ($inherits: ty, $ty: ty, $value: ident => $decode_to: expr, $value1: ident => $encode_to: expr) => {
        impl $crate::encoding::HasMaxEncodeSize for $ty {
            const ENCODE_SIZE: usize = size_of::<$inherits>();
        }
        impl $crate::encoding::EncodeSizeKnown for $ty {}
        impl $crate::encoding::MessageParam for $ty {
            #[inline(always)]
            fn encode_size(&self) -> usize {
                size_of::<$inherits>()
            }

            #[inline(always)]
            fn encode_into_buf(
                &self,
                buf: &mut [u8],
            ) -> Result<usize, crate::encoding::BufferTooSmall> {
                let $value1 = self;
                ($encode_to).encode_into_buf(buf)
            }

            #[inline(always)]
            fn encode_into<W: std::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<usize, std::io::Error> {
                let $value1 = self;
                ($encode_to).encode_into(writer)
            }

            #[inline(always)]
            fn decode_from_buf(buf: &[u8]) -> Result<(Self, usize), crate::encoding::DecodeError> {
                let ($value, read) = <$inherits>::decode_from_buf(buf)?;

                Ok(($decode_to, read))
            }

            #[inline(always)]
            fn decode_from<R: std::io::Read>(
                reader: &mut R,
            ) -> Result<(Self, usize), crate::DecodeErrorOrIo> {
                let ($value, read) = <$inherits>::decode_from(reader)?;
                Ok(($decode_to, read))
            }
        }
    };
}

pub use impl_inheritly;
use thiserror::Error;

impl_inheritly!(u8, bool, from_u8 => {
    match from_u8 {
        0 => false,
        1 => true,
        _ => return Err(crate::encoding::DecodeError::InvalidData.into()),
    }
}, from_self => *from_self as u8);

impl<const N: usize, T: EncodeSizeKnown> HasMaxEncodeSize for [T; N] {}
impl<const N: usize, T: EncodeSizeKnown> EncodeSizeKnown for [T; N] {}
