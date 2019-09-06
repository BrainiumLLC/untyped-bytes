use std::{borrow::Borrow, mem, slice};

#[derive(Clone, Debug, Default)]
pub struct UntypedBytes {
    bytes: Vec<u8>,
}

// unsafe to inspect the bytes after casting
unsafe fn as_bytes<T: Copy + 'static>(value: &T) -> &[u8] {
    slice::from_raw_parts(value as *const T as _, mem::size_of::<T>())
}

// unsafe to inspect the bytes after casting
unsafe fn as_bytes_slice<T: Copy + 'static>(value: &[T]) -> &[u8] {
    slice::from_raw_parts(value.as_ptr() as _, mem::size_of_val(value))
}

impl UntypedBytes {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    /// Effectively a `mem::transmute`.
    pub fn from_vec<T: Copy + 'static>(mut value: Vec<T>) -> Self {
        let size = mem::size_of::<T>();
        let bytes = unsafe {
            Vec::from_raw_parts(
                value.as_mut_ptr() as _,
                value.len() * size,
                value.capacity() * size,
            )
        };
        mem::forget(value);
        Self { bytes }
    }

    pub fn from_slice<T, V>(value: V) -> Self
    where
        T: Copy + 'static,
        V: Borrow<[T]>,
    {
        let borrowed = value.borrow();
        let mut result = Self::with_capacity(mem::size_of_val(borrowed));
        let raw = unsafe { as_bytes_slice(borrowed) };
        result.bytes.extend(raw);
        result
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn clear(&mut self) {
        self.bytes.clear()
    }

    pub fn push<T: Copy + 'static>(&mut self, value: T) {
        let raw = unsafe { as_bytes(&value) };
        self.bytes.extend(raw)
    }

    pub fn extend_from_slice<T, V>(&mut self, value: V)
    where
        T: Copy + 'static,
        V: Borrow<[T]>,
    {
        let raw = unsafe { as_bytes_slice(value.borrow()) };
        self.bytes.extend_from_slice(raw)
    }

    /// Returns a slice that is unsafe to inspect in the presence of padding bytes, but is safe to
    /// `memcpy`.
    pub unsafe fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
}

impl<T: Copy + 'static> From<T> for UntypedBytes {
    fn from(value: T) -> Self {
        let mut result = Self::with_capacity(mem::size_of::<T>());
        result.push(value);
        result
    }
}

impl<A: Copy + 'static> Extend<A> for UntypedBytes {
    fn extend<T: IntoIterator<Item = A>>(&mut self, value: T) {
        self.bytes
            .extend(value.into_iter().flat_map(|value| ByteIter::from(value)))
    }
}

#[derive(Clone)]
struct ByteIter<T: Copy + 'static> {
    value: T,
    cur: usize,
}

impl<T: Copy + 'static> From<T> for ByteIter<T> {
    fn from(value: T) -> Self {
        Self { value, cur: 0 }
    }
}

impl<T: Copy + 'static> Iterator for ByteIter<T> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.cur < mem::size_of::<T>() {
            let prev = self.cur;
            self.cur += 1;
            Some(unsafe { as_bytes(&self.value).get_unchecked(prev).clone() })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = mem::size_of::<T>() - self.cur;
        (size, Some(size))
    }
}

impl<T: Copy + 'static> ExactSizeIterator for ByteIter<T> {}
