use crate::errors::Result;
use crate::stringhandle::StringArray;
use std::borrow::Cow;

pub struct DataArray<'a, T>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    data: Cow<'a, [T]>,
    sizes: Cow<'a, [i32]>,
}
impl<'a, T> DataArray<'a, T>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    pub fn new(data: &'a [T], sizes: &'a [i32]) -> DataArray<'a, T> {
        debug_assert_eq!(sizes.iter().sum::<i32>() as usize, data.len());
        DataArray {
            data: Cow::Borrowed(data),
            sizes: Cow::Borrowed(sizes),
        }
    }

    pub(crate) fn new_owned(data: Vec<T>, sizes: Vec<i32>) -> DataArray<'static, T> {
        debug_assert_eq!(sizes.iter().sum::<i32>() as usize, data.len());
        DataArray {
            data: Cow::Owned(data),
            sizes: Cow::Owned(sizes),
        }
    }

    pub fn data(&self) -> &[T] {
        self.data.as_ref()
    }

    pub fn data_mut(&mut self) -> &mut [T] {
        self.data.to_mut().as_mut()
    }
    pub fn sizes(&self) -> &[i32] {
        self.sizes.as_ref()
    }

    pub fn iter(&'a self) -> ArrayIter<'a, T> {
        ArrayIter {
            sizes: self.sizes.iter(),
            data: self.data.as_ref(),
            cursor: 0,
        }
    }
    pub fn iter_mut(&'a mut self) -> ArrayIterMut<'a, T> {
        ArrayIterMut {
            sizes: self.sizes.to_mut().iter_mut(),
            data: self.data.to_mut().as_mut(),
            cursor: 0,
        }
    }
}

pub struct StringMultiArray {
    pub(crate) handles: Vec<i32>,
    pub(crate) sizes: Vec<i32>,
    pub(crate) session: crate::session::Session,
}

pub struct ArrayIter<'a, T> {
    data: &'a [T],
    sizes: std::slice::Iter<'a, i32>,
    cursor: usize,
}

pub struct ArrayIterMut<'a, T> {
    data: &'a mut [T],
    sizes: std::slice::IterMut<'a, i32>,
    cursor: usize,
}

pub struct MultiArrayIter<'a> {
    handles: std::slice::Iter<'a, i32>,
    sizes: std::slice::Iter<'a, i32>,
    session: &'a crate::session::Session,
    cursor: usize,
}

impl<'a, T> Iterator for ArrayIter<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        match self.sizes.next() {
            None => None,
            Some(size) => {
                let start = self.cursor;
                let end = self.cursor + (*size as usize);
                self.cursor = end;
                // SAFETY: The data and the sizes arrays are both provided by HAPI
                // are expected to match. Also bounds are checked in debug build.
                Some(unsafe { self.data.get_unchecked(start..end) })
            }
        }
    }
}

impl<'a, T> Iterator for ArrayIterMut<'a, T> {
    type Item = &'a mut [T];

    fn next(&mut self) -> Option<Self::Item> {
        match self.sizes.next() {
            None => None,
            Some(size) => {
                let start = self.cursor;
                let end = self.cursor + (*size as usize);
                self.cursor = end;
                // SAFETY: Compiler can't know that we're never return overlapping references
                // so we "erase" the lifetime by casting to pointer and back.
                Some(unsafe { &mut *(self.data.get_unchecked_mut(start..end) as *mut [T]) })
            }
        }
    }
}

impl StringMultiArray {
    pub fn iter(&self) -> MultiArrayIter<'_> {
        MultiArrayIter {
            handles: self.handles.iter(),
            sizes: self.sizes.iter(),
            session: &self.session,
            cursor: 0,
        }
    }
}

impl<'a> Iterator for MultiArrayIter<'a> {
    type Item = Result<StringArray>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.sizes.next() {
            None => None,
            Some(size) => {
                let start = self.cursor;
                let end = self.cursor + (*size as usize);
                self.cursor = end;
                let handles = &self.handles.as_slice()[start..end];
                Some(crate::stringhandle::get_string_array(handles, self.session))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_array_iter() {
        let ar = DataArray::new_owned(vec![1, 2, 3, 4, 5, 6], vec![2, 1, 3]);
        let mut iter = ar.iter();
        assert_eq!(iter.next(), Some([1, 2].as_slice()));
        assert_eq!(iter.next(), Some([3].as_slice()));
        assert_eq!(iter.next(), Some([4, 5, 6].as_slice()));
    }

    #[test]
    fn data_array_mutate() {
        let mut ar = DataArray::new(&[1, 2, 3, 4, 5, 6], &[2, 1, 3]);
        let mut iter = ar.iter_mut().map(|v| {
            v.iter_mut().for_each(|v| *v *= 2);
            v
        });
        assert_eq!(iter.next(), Some([2, 4].as_mut_slice()));
        assert_eq!(iter.next(), Some([6].as_mut_slice()));
        assert_eq!(iter.next(), Some([8, 10, 12].as_mut_slice()));
    }
}
