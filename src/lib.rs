use std::{
    borrow::Borrow,
    mem::{self, MaybeUninit},
    slice,
};

#[derive(Clone, Debug, Default)]
pub struct UntypedBytes {
    bytes: Vec<u8>,
}

// unsafe to inspect the bytes after casting
#[inline]
unsafe fn as_bytes<T: Copy + Send + Sync + 'static>(value: &T) -> &[u8] {
    slice::from_raw_parts(value as *const T as _, mem::size_of::<T>())
}

// unsafe to inspect the bytes after casting
#[inline]
unsafe fn as_bytes_slice<T: Copy + Send + Sync + 'static>(value: &[T]) -> &[u8] {
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
        T: Copy + Send + Sync + 'static,
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

    pub fn push<T: Copy + Send + Sync + 'static>(&mut self, value: T) {
        let raw = unsafe { as_bytes(&value) };
        self.bytes.extend(raw)
    }

    #[inline]
    pub fn extend_from_slice<T, V>(&mut self, value: V)
    where
        T: Copy + Send + Sync + 'static,
        V: Borrow<[T]>,
    {
        let raw = unsafe { as_bytes_slice(value.borrow()) };
        self.bytes.extend_from_slice(raw)
    }

    /// Returns a slice that is unsafe to inspect in the presence of padding bytes, but is safe to
    /// `memcpy`. Additionally, alignment of the returned slice is the same as
    /// `mem::align_of::<u8>()`.
    pub unsafe fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Casts the backing bytes to a value of type `T`. This is only safe the backing bytes were
    /// created from a value of type `T`.
    pub unsafe fn cast<T: Copy + Send + Sync + 'static>(&self) -> T {
        debug_assert_eq!(
            mem::size_of::<T>(),
            self.len(),
            "Attempt to cast `UntypedBytes` to a value of a different size"
        );
        let mut result = MaybeUninit::uninit();
        self.as_slice()
            .as_ptr()
            .copy_to_nonoverlapping(result.as_mut_ptr() as *mut u8, mem::size_of::<T>());
        result.assume_init()
    }
}

impl<T: Copy + Send + Sync + 'static> From<T> for UntypedBytes {
    fn from(value: T) -> Self {
        Self::from_vec(vec![value])
    }
}

impl<A: Copy + Send + Sync + 'static> Extend<A> for UntypedBytes {
    #[inline]
    fn extend<T: IntoIterator<Item = A>>(&mut self, value: T) {
        if std::any::type_name::<T>()
            == std::any::type_name::<std::iter::Copied<std::slice::Iter<'_, A>>>()
        {
            let raw = unsafe { mem::transmute_copy::<_, std::slice::Iter<'_, A>>(&value) };
            self.extend_from_slice(raw.as_slice())
        } else if std::any::type_name::<T>()
            == std::any::type_name::<std::iter::Cloned<std::slice::Iter<'_, A>>>()
        {
            let raw = unsafe { mem::transmute_copy::<_, std::slice::Iter<'_, A>>(&value) };
            self.extend_from_slice(raw.as_slice())
        } else if std::any::type_name::<T>()
            == std::any::type_name::<std::iter::Copied<std::slice::IterMut<'_, A>>>()
        {
            let raw = unsafe { mem::transmute_copy::<_, std::slice::IterMut<'_, A>>(&value) };
            self.extend_from_slice(raw.into_slice())
        } else if std::any::type_name::<T>()
            == std::any::type_name::<std::iter::Cloned<std::slice::IterMut<'_, A>>>()
        {
            let raw = unsafe { mem::transmute_copy::<_, std::slice::IterMut<'_, A>>(&value) };
            self.extend_from_slice(raw.into_slice())
        } else if std::any::type_name::<T>() == std::any::type_name::<std::vec::IntoIter<A>>() {
            let raw = unsafe { mem::transmute_copy::<_, std::vec::IntoIter<A>>(&value) };
            std::mem::forget(value);
            self.extend_from_slice(raw.as_slice())
        } else if std::any::type_name::<T>() == std::any::type_name::<Vec<A>>() {
            let raw = unsafe { mem::transmute_copy::<_, Vec<A>>(&value) };
            std::mem::forget(value);
            self.extend_from_slice(raw.as_slice())
        } else {
            for elem in value {
                self.push(elem)
            }
        }
    }
}
