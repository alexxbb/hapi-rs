use crate::errors::{HapiError, Result};
use crate::stringhandle::{StringArray, StringHandle};

/// Owned values and per-element sizes returned by HAPI array attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct JaggedArrayData<T> {
    data: Vec<T>,
    sizes: Vec<i32>,
}

impl<T> JaggedArrayData<T> {
    pub fn new(data: Vec<T>, sizes: Vec<i32>) -> Result<Self> {
        validate_sizes(&sizes, data.len())?;
        Ok(Self { data, sizes })
    }

    pub(crate) fn from_hapi(data: Vec<T>, sizes: Vec<i32>) -> Result<Self> {
        Self::new(data, sizes)
    }

    #[must_use]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    #[must_use]
    pub fn sizes(&self) -> &[i32] {
        &self.sizes
    }

    #[must_use]
    pub fn iter(&self) -> JaggedArrayIter<'_, T> {
        JaggedArrayIter {
            data: &self.data,
            sizes: self.sizes.iter(),
            cursor: 0,
        }
    }
}

impl<'a, T> IntoIterator for &'a JaggedArrayData<T> {
    type Item = &'a [T];
    type IntoIter = JaggedArrayIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct JaggedArrayIter<'a, T> {
    data: &'a [T],
    sizes: std::slice::Iter<'a, i32>,
    cursor: usize,
}

impl<'a, T> Iterator for JaggedArrayIter<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<Self::Item> {
        let size = usize::try_from(*self.sizes.next()?).ok()?;
        let end = self.cursor.checked_add(size)?;
        let result = self.data.get(self.cursor..end)?;
        self.cursor = end;
        Some(result)
    }
}

/// Jagged string or dictionary data. Each item is exposed as a `StringArray`.
#[derive(Debug, Clone)]
pub struct StringJaggedArrayData {
    pub(crate) handles: Vec<StringHandle>,
    pub(crate) sizes: Vec<i32>,
    pub(crate) session: debug_ignore::DebugIgnore<crate::session::Session>,
}

impl StringJaggedArrayData {
    pub(crate) fn from_hapi(
        handles: Vec<StringHandle>,
        sizes: Vec<i32>,
        session: crate::session::Session,
    ) -> Result<Self> {
        validate_sizes(&sizes, handles.len())?;
        Ok(Self {
            handles,
            sizes,
            session: debug_ignore::DebugIgnore(session),
        })
    }

    #[must_use]
    pub fn sizes(&self) -> &[i32] {
        &self.sizes
    }

    #[must_use]
    pub fn iter(&self) -> StringJaggedArrayIter<'_> {
        StringJaggedArrayIter {
            handles: &self.handles,
            sizes: self.sizes.iter(),
            session: &self.session,
            cursor: 0,
        }
    }

    pub fn flatten(self) -> Result<(Vec<String>, Vec<usize>)> {
        let mut flat = Vec::with_capacity(self.handles.len());
        for item in &self {
            flat.extend(item?);
        }
        let sizes = self.sizes.iter().map(|&v| v as usize).collect();
        Ok((flat, sizes))
    }
}

impl<'a> IntoIterator for &'a StringJaggedArrayData {
    type Item = Result<StringArray>;
    type IntoIter = StringJaggedArrayIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct StringJaggedArrayIter<'a> {
    handles: &'a [StringHandle],
    sizes: std::slice::Iter<'a, i32>,
    session: &'a crate::session::Session,
    cursor: usize,
}

impl Iterator for StringJaggedArrayIter<'_> {
    type Item = Result<StringArray>;
    fn next(&mut self) -> Option<Self::Item> {
        let size = usize::try_from(*self.sizes.next()?).ok()?;
        let end = self.cursor.checked_add(size)?;
        let handles = self.handles.get(self.cursor..end)?;
        self.cursor = end;
        Some(crate::stringhandle::get_string_array(handles, self.session))
    }
}

fn validate_sizes(sizes: &[i32], data_len: usize) -> Result<()> {
    let mut total = 0usize;
    for (index, &size) in sizes.iter().enumerate() {
        let size = usize::try_from(size).map_err(|_| {
            HapiError::Internal(format!(
                "jagged array size at index {index} is negative: {size}"
            ))
        })?;
        total = total.checked_add(size).ok_or_else(|| {
            HapiError::Internal("jagged array size total overflowed usize".into())
        })?;
    }
    if total != data_len {
        return Err(HapiError::Internal(format!(
            "jagged array size total ({total}) does not match data length ({data_len})"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jagged_validation_and_iteration() {
        assert!(JaggedArrayData::new(vec![1], vec![-1]).is_err());
        assert!(JaggedArrayData::new(vec![1], vec![2]).is_err());
        let value = JaggedArrayData::new(vec![1, 2, 3], vec![2, 1]).unwrap();
        assert_eq!(
            value.iter().collect::<Vec<_>>(),
            vec![&[1, 2][..], &[3][..]]
        );
    }
}
