use crate::errors::{HapiError, Result};

/// Owned flattened values and per-element sizes for a HAPI array attribute.
///
/// The `sizes` array contains one entry for every geometry element owned by the
/// attribute. Each size describes the corresponding slice in `data`.
#[derive(Debug, Clone, PartialEq)]
pub struct JaggedArrayData<T> {
    data: Vec<T>,
    sizes: Vec<i32>,
}

impl<T> JaggedArrayData<T> {
    /// Creates validated jagged attribute data.
    ///
    /// Returns an error when a size is negative, the total overflows `usize`,
    /// or the sum of all sizes differs from `data.len()`.
    pub fn new(data: Vec<T>, sizes: Vec<i32>) -> Result<Self> {
        validate_sizes(&sizes, data.len())?;
        Ok(Self { data, sizes })
    }

    pub(crate) fn from_hapi(data: Vec<T>, sizes: Vec<i32>) -> Result<Self> {
        Self::new(data, sizes)
    }

    /// Returns the flattened attribute values.
    #[must_use]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Returns the number of values belonging to each geometry element.
    #[must_use]
    pub fn sizes(&self) -> &[i32] {
        &self.sizes
    }

    /// Iterates over the bounds-checked slice for each geometry element.
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

/// Bounds-checked iterator over the entries of [`JaggedArrayData`].
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

/// Owned jagged string or dictionary attribute data.
///
/// HAPI string handles are resolved before this value is returned, so reading
/// or iterating it never performs another HAPI call.
#[derive(Debug, Clone, PartialEq)]
pub struct StringJaggedArrayData {
    data: Vec<String>,
    sizes: Vec<i32>,
}

impl StringJaggedArrayData {
    /// Creates validated, owned jagged string data.
    pub fn new(data: Vec<String>, sizes: Vec<i32>) -> Result<Self> {
        validate_sizes(&sizes, data.len())?;
        Ok(Self { data, sizes })
    }

    pub(crate) fn from_hapi(data: Vec<String>, sizes: Vec<i32>) -> Result<Self> {
        Self::new(data, sizes)
    }

    /// Returns the flattened string values.
    #[must_use]
    pub fn data(&self) -> &[String] {
        &self.data
    }

    /// Returns the number of strings belonging to each geometry element.
    #[must_use]
    pub fn sizes(&self) -> &[i32] {
        &self.sizes
    }

    /// Iterates over each geometry element's strings.
    #[must_use]
    pub fn iter(&self) -> StringJaggedArrayIter<'_> {
        StringJaggedArrayIter {
            data: &self.data,
            sizes: self.sizes.iter(),
            cursor: 0,
        }
    }

    /// Consumes the value and returns its flattened strings and sizes.
    #[must_use]
    pub fn into_parts(self) -> (Vec<String>, Vec<i32>) {
        (self.data, self.sizes)
    }
}

impl<'a> IntoIterator for &'a StringJaggedArrayData {
    type Item = &'a [String];
    type IntoIter = StringJaggedArrayIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Bounds-checked iterator over [`StringJaggedArrayData`].
pub struct StringJaggedArrayIter<'a> {
    data: &'a [String],
    sizes: std::slice::Iter<'a, i32>,
    cursor: usize,
}

impl<'a> Iterator for StringJaggedArrayIter<'a> {
    type Item = &'a [String];
    fn next(&mut self) -> Option<Self::Item> {
        let size = usize::try_from(*self.sizes.next()?).ok()?;
        let end = self.cursor.checked_add(size)?;
        let values = self.data.get(self.cursor..end)?;
        self.cursor = end;
        Some(values)
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

    #[test]
    fn owned_string_jagged_validation_and_iteration() {
        let value = StringJaggedArrayData::new(
            vec!["one".into(), "two".into(), "three".into()],
            vec![2, 1],
        )
        .unwrap();
        assert_eq!(value.data(), ["one", "two", "three"]);
        assert_eq!(
            value.iter().collect::<Vec<_>>(),
            vec![&value.data()[..2], &value.data()[2..]]
        );
    }
}
