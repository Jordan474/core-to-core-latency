use std::ptr::NonNull;
use std::ops::{Deref, DerefMut};

#[cfg(unix)]
mod unix;

#[cfg(unix)]
use unix::allocate_membind_here;

pub struct NumaBox<T>(NonNull<T>);

/// A simple Box, that allocates into a memory bound to the current numa node,
/// if available.
impl<T> NumaBox<T> {
    pub fn new_membind_here(x: T) -> Self {
        Self(allocate_membind_here(x))
    }
}

impl<T> Deref for NumaBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { & *self.0.as_ptr() }
    }
}

impl<T> DerefMut for NumaBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.as_ptr() }
    }
}

