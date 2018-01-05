use std::ops::{Index, IndexMut};

// A vec that pushes from the left instead of the right but iterates
// in the same order as a regular vec so that we can avoid reversal.

pub struct RevVec<T> {
    inner: Vec<T>,
}

impl<T> RevVec<T> {
    pub fn push(&mut self, t: T) {
        self.inner.push(t);
    }

    pub fn new() -> RevVec<T> {
        RevVec { inner: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> RevVec<T> {
        RevVec {
            inner: Vec::with_capacity(cap),
        }
    }
}

impl<T> Index<usize> for RevVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.inner[self.inner.len() - index - 1]
    }
}

impl<T> IndexMut<usize> for RevVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        let len = self.inner.len();
        &mut self.inner[len - index - 1]
    }
}

impl<T> IntoIterator for RevVec<T> {
    type Item = T;
    type IntoIter = ::std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
