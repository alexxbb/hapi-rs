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
    fn new(dat: &'a [T], sizes: &'a [i32]) -> DataArray<'a, T> {
        DataArray {
            data: Cow::Borrowed(dat),
            sizes: Cow::Borrowed(sizes),
        }
    }

    pub(crate) fn new_owned(dat: Vec<T>, sizes: Vec<i32>) -> DataArray<'static, T> {
        DataArray {
            data: Cow::Owned(dat),
            sizes: Cow::Owned(sizes),
        }
    }

    fn data(&self) -> &[T] {
        self.data.as_ref()
    }

    fn data_mut(&mut self) -> &mut [T] {
        self.data.to_mut().as_mut()
    }
    fn sizes(&self) -> &[i32] {
        self.sizes.as_ref()
    }

    fn iter_values(&'a self) -> ArrayIter<'a, T> {
        ArrayIter {
            sizes: self.sizes.iter(),
            data: self.data.iter(),
            cursor: 0,
        }
    }
}

pub struct StringMultiArray {
    pub handles: Vec<i32>,
    pub sizes: Vec<i32>,
    pub(crate) session: crate::session::Session,
}

pub struct ArrayIter<'a, T> {
    data: std::slice::Iter<'a, T>,
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
                // TODO: We know the data size, it can be rewritten to use unsafe unchecked
                Some(&self.data.as_slice()[start..end])
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
                Some(&self.data[start..end])
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
        let mut iter = ar.iter_values();
        assert_eq!(iter.next(), Some([1, 2].as_slice()));
        assert_eq!(iter.next(), Some([3].as_slice()));
        assert_eq!(iter.next(), Some([4, 5, 6].as_slice()));
    }

    #[test]
    fn data_array_mutate() {
        let mut ar = DataArray::new(&[1, 2, 3, 4, 5, 6], &[2, 1, 3]);
        let new: Vec<_> = ar.data_mut().iter_mut().map(|v| *v * 2).collect();
        assert_eq!(new, &[2, 4, 6, 8, 10, 16])
    }
}