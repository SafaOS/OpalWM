use std::{
    fmt::{Debug, Display},
    hash::Hash,
    mem::MaybeUninit,
    ops::Deref,
};

use crate::encoding::{
    BufferTooSmall, DecodeError, DecodeErrorOrIo, HasMaxEncodeSize, MessageParam,
};

/// An array of maximum size `MAX`.
#[derive(Copy)]
pub struct BufOfMax<const MAX: usize, T: Copy> {
    len: usize,
    inner: [MaybeUninit<T>; MAX],
}

impl<const MAX: usize, T: Copy> BufOfMax<MAX, T> {
    /// Constructs a new empty buffer.
    #[inline(always)]
    pub const fn new_empty() -> Self {
        Self {
            len: 0,
            inner: [const { MaybeUninit::uninit() }; MAX],
        }
    }

    /// Constructs a new buffer filled with the given values.
    #[inline(always)]
    pub fn new_filled(values: [T; MAX]) -> Self {
        let uninit: [MaybeUninit<T>; MAX] = core::array::from_fn(|i| MaybeUninit::new(values[i]));
        Self {
            len: MAX,
            inner: uninit,
        }
    }

    /// Returns the length of the buffer.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the buffer is empty.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a slice of the buffer's items.
    #[inline(always)]
    #[track_caller]
    pub const fn items(&self) -> &[T] {
        let r = unsafe { core::mem::transmute::<&[MaybeUninit<T>; MAX], &[T; MAX]>(&self.inner) };
        let (items, _) = unsafe { r.split_at_unchecked(self.len) };
        items
    }

    /// Pushes a given item to the end of the buffer, returning an Err with it if the buffer is full.
    #[inline]
    #[track_caller]
    pub const fn push(&mut self, item: T) -> Result<(), T> {
        if self.len() + 1 > MAX {
            Err(item)
        } else {
            self.inner[self.len].write(item);
            self.len += 1;
            Ok(())
        }
    }

    /// Pushes a given itemS to the end of the buffer, returning an Err if the buffer cannot fit all of the given elements.
    #[inline]
    #[track_caller]
    pub const fn push_all(&mut self, items: &[T]) -> Result<(), ()> {
        if self.len() + items.len() > MAX {
            Err(())
        } else {
            let mut i = 0;
            while i < items.len() {
                self.inner[self.len].write(items[i]);
                self.len += 1;
                i += 1;
            }
            Ok(())
        }
    }
}

impl<const N: usize, T: Copy + Debug> Debug for BufOfMax<N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.items().fmt(f)
    }
}

impl<const N: usize, T: HasMaxEncodeSize> HasMaxEncodeSize for BufOfMax<N, T> {
    const ENCODE_SIZE: usize = size_of::<[T; N]>();
}

impl<const N: usize, T: Copy> Clone for BufOfMax<N, T> {
    fn clone(&self) -> Self {
        let len = self.len;
        let values: [MaybeUninit<T>; N] = core::array::from_fn(|i| {
            if i < len {
                MaybeUninit::new(unsafe { self.inner[i].assume_init_ref().clone() })
            } else {
                MaybeUninit::uninit()
            }
        });

        Self { len, inner: values }
    }
}

impl<const MAX: usize, T: Copy> Default for BufOfMax<MAX, T> {
    fn default() -> Self {
        Self::new_empty()
    }
}

impl<const N: usize, T: Copy> Deref for BufOfMax<N, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.items()
    }
}

impl<const MAX: usize, T: MessageParam + HasMaxEncodeSize> MessageParam for BufOfMax<MAX, T> {
    #[inline(always)]
    fn encode_size(&self) -> usize {
        (self.len * size_of::<T>()) + size_of::<usize>()
    }

    #[inline]
    fn encode_into<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        self.len().encode_into(writer)?;
        for item in &**self {
            item.encode_into(writer)?;
        }

        Ok(self.encode_size())
    }

    #[inline]
    fn decode_from<R: std::io::Read>(reader: &mut R) -> Result<(Self, usize), DecodeErrorOrIo> {
        let (len, _) = usize::decode_from(reader)?;
        if len > MAX {
            return Err(DecodeErrorOrIo::DecodeError(DecodeError::InvalidData));
        }

        let mut items = BufOfMax::new_empty();
        for _ in 0..len {
            let (item, _) = T::decode_from(reader)?;
            _ = items.push(item);
        }

        let encode_size = items.encode_size();
        Ok((items, encode_size))
    }

    #[inline]
    fn encode_into_buf(&self, buf: &mut [u8]) -> Result<usize, BufferTooSmall> {
        if buf.len() < self.encode_size() {
            return Err(BufferTooSmall);
        }

        let (len_side, rest_of_data) = buf.split_at_mut(size_of::<usize>());
        len_side.copy_from_slice(&self.len.to_ne_bytes());

        let mut current = rest_of_data;
        for item in self.items() {
            let size = item.encode_into_buf(current).expect("Item is Var Length");
            current = &mut current[size..];
        }
        Ok(self.encode_size())
    }

    #[inline]
    fn decode_from_buf(buf: &[u8]) -> Result<(Self, usize), DecodeError> {
        if buf.len() < size_of::<usize>() {
            return Err(DecodeError::BufferTooSmall);
        }

        let (len_side, rest_of_data) = buf.split_at(size_of::<usize>());
        let len: usize = bytemuck::pod_read_unaligned(len_side);

        if len > rest_of_data.len() / size_of::<T>() {
            return Err(DecodeError::BufferTooSmall);
        }

        let mut result = BufOfMax::new_empty();
        let mut current = rest_of_data;

        for _ in 0..len {
            let (item, size) = T::decode_from_buf(current)?;
            result.push(item).map_err(|_| DecodeError::InvalidData)?;
            current = &current[size..];
        }

        let final_size = result.encode_size();
        Ok((result, final_size))
    }
}

impl<const MAX: usize, T: PartialEq + Copy> PartialEq for BufOfMax<MAX, T> {
    fn eq(&self, other: &Self) -> bool {
        self.items() == other.items()
    }
}
impl<const MAX: usize, T: PartialEq + Eq + Copy> Eq for BufOfMax<MAX, T> {}
impl<const MAX: usize, T: Hash + Copy> Hash for BufOfMax<MAX, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.items().hash(state);
    }
}

/// Same as [`BufOfMax<MAX, u8>`], except that it is guaranteed to be a valid UTF-8 string.
#[derive(Clone, Copy, PartialEq, Default, Eq)]
pub struct StrOfMax<const MAX: usize>(BufOfMax<MAX, u8>);

impl<const MAX: usize> StrOfMax<MAX> {
    /// Creates a new [`StrOfMax`] from a buffer, returning an Err if the buffer is not a valid UTF-8 string.
    #[inline]
    pub const fn from_buf(buf: BufOfMax<MAX, u8>) -> Result<Self, std::str::Utf8Error> {
        if let Err(e) = std::str::from_utf8(buf.items()) {
            return Err(e);
        }
        Ok(StrOfMax(buf))
    }

    /// Creates a new [`StrOfMax`] from a string slice, truncating the string if it is too long.
    #[inline]
    pub fn new_truncate(s: &str) -> Self {
        let used_str = 'blk: {
            if s.len() <= MAX {
                break 'blk s;
            }

            let mut end = MAX;
            while !s.is_char_boundary(end) {
                end -= 1;
            }

            &s[..end]
        };

        StrOfMax::new(used_str).expect("Should never fail")
    }

    /// Creates a new [`StrOfMax`] from a string slice, returning an Err if the string is too long.
    #[inline(always)]
    pub const fn new(s: &str) -> Result<Self, ()> {
        let mut new_s = StrOfMax::new_empty();
        if let Err(()) = new_s.push_str(s) {
            return Err(());
        }
        Ok(new_s)
    }

    #[inline(always)]
    /// Creates a new empty [`StrOfMax`].
    pub const fn new_empty() -> Self {
        StrOfMax(BufOfMax::new_empty())
    }

    #[inline(always)]
    /// Returns the string slice contained in this [`StrOfMax`].
    pub const fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.0.items()) }
    }

    #[inline(always)]
    /// Returns the length of the string contained in this [`StrOfMax`], in bytes.
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline(always)]
    /// Appends a string slice to the end of this [`StrOfMax`], returning an Err if the string is too long.
    pub const fn push_str(&mut self, s: &str) -> Result<(), ()> {
        self.0.push_all(s.as_bytes())
    }

    #[inline(always)]
    /// Appends a character to the end of this [`StrOfMax`], returning an Err if the string is too long.
    pub const fn push_char(&mut self, c: char) -> Result<(), ()> {
        let mut tmp_buf = [0; 4];
        let s = c.encode_utf8(&mut tmp_buf);
        self.push_str(s)
    }
}

impl<const N: usize> Debug for StrOfMax<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl<const N: usize> Display for StrOfMax<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl<const MAX: usize> HasMaxEncodeSize for StrOfMax<MAX> {}
impl<const MAX: usize> MessageParam for StrOfMax<MAX> {
    #[inline(always)]
    fn encode_size(&self) -> usize {
        BufOfMax::encode_size(&self.0)
    }

    #[inline(always)]
    fn decode_from<R: std::io::Read>(reader: &mut R) -> Result<(Self, usize), DecodeErrorOrIo> {
        let (buf, size) = BufOfMax::decode_from(reader)?;
        StrOfMax::from_buf(buf)
            .map_err(|_| DecodeErrorOrIo::DecodeError(DecodeError::InvalidData))
            .map(|s| (s, size))
    }

    #[inline(always)]
    fn decode_from_buf(buf: &[u8]) -> Result<(Self, usize), DecodeError> {
        let (buf, size) = BufOfMax::decode_from_buf(buf)?;
        StrOfMax::from_buf(buf)
            .map_err(|_| DecodeError::InvalidData)
            .map(|s| (s, size))
    }

    #[inline(always)]
    fn encode_into<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        BufOfMax::encode_into(&self.0, writer)
    }
    #[inline(always)]
    fn encode_into_buf(&self, buf: &mut [u8]) -> Result<usize, BufferTooSmall> {
        BufOfMax::encode_into_buf(&self.0, buf)
    }
}

impl<const MAX: usize> Deref for StrOfMax<MAX> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
mod test {
    use crate::{BufOfMax, encoding::MessageParam, misc::StrOfMax};

    #[test]
    fn buf_of_max_construct() {
        let mut buf = BufOfMax::<4, u32>::new_empty();
        assert!(buf.is_empty());
        buf.push(0xAD).expect("Buffer should have enough space");
        assert_eq!(&*buf, &[0xAD]);

        buf.push_all(&[0xA2, 0xB6])
            .expect("Buffer should have enough space");
        assert_eq!(&*buf, &[0xAD, 0xA2, 0xB6]);

        buf.push_all(&[0x22, 0x33])
            .expect_err("Buffer shouldn't have enough space");
    }

    #[test]
    fn str_of_max_construct() {
        let mut s = StrOfMax::<10>::new("Fit!").expect("Buffer should have enough space");
        assert_eq!(&*s, "Fit!");

        s.push_char(' ').expect("Buffer should have enough space");
        assert_eq!(&*s, "Fit! ");

        s.push_str("Too Big for that!")
            .expect_err("Buffer shouldn't have enough space");

        s.push_str("Now").expect("Buffer should have enough space");
        assert_eq!(&*s, "Fit! Now");

        StrOfMax::<2>::new("Too big!").expect_err("Buffer shouldn't have enough space");
    }

    #[test]
    fn str_of_max_encoding() {
        assert_eq!(size_of::<usize>(), 8, "Test won't work on platform");

        let s = StrOfMax::<6>::new("Hello!").expect("String should have enough space");
        let mut buf = [0u8; 6 + size_of::<usize>()];
        let size = s
            .encode_into_buf(&mut buf)
            .expect("Buffer should have enough space");
        assert_eq!(
            size,
            6 + size_of::<usize>(),
            "Buffer encoding size mismatch"
        );

        assert_eq!(
            &buf,
            &[6, 0, 0, 0, 0, 0, 0, 0, b'H', b'e', b'l', b'l', b'o', b'!']
        );

        let (got, size) = StrOfMax::<10>::decode_from_buf(&buf).expect("Decode error");
        assert_eq!(
            size,
            6 + size_of::<usize>(),
            "Buffer decoding size mismatch"
        );
        assert_eq!(&*got, "Hello!");
    }
}

/// The maximum length of an Object's name
pub const MAX_NAME_LEN: usize = 128;

/// Describes A Generic Name for an object, stored in the stack
///
/// Alias for [`StrOfMax`], with a maximum length of [`MAX_NAME_LEN`].
pub type Name = StrOfMax<{ MAX_NAME_LEN }>;
