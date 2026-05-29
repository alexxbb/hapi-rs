//! String handling utilities and iterators
use std::ffi::{CStr, CString};
use std::fmt::Formatter;

use crate::errors::Result;
use crate::session::Session;

// StringArray iterators SAFETY: Are Houdini strings expected to be valid utf? Maybe revisit.

/// A handle to a string returned by some api.
/// Then the String can be retrieved with [`Session::get_string`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct StringHandle(pub(crate) i32);

/// Holds a contiguous array of bytes where each individual string value is null-separated.
/// You can choose how to iterate over it by calling a corresponding iter_* function.
/// The `Debug` impl has an alternative `{:#?}` representation, which prints as a vec of strings.
#[derive(Clone, PartialEq)]
pub struct StringArray(pub Vec<u8>);

impl std::fmt::Debug for StringArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            let strings = self.iter_str().collect::<Vec<_>>();
            strings.fmt(f)
        } else {
            let count = bytecount::count(self.0.as_slice(), b'\0');
            write!(f, "StringArray[num_strings = {count}]")
        }
    }
}

/// Iterator over &str, returned from `StringArray::iter_str()`
pub struct StringIter<'a> {
    inner: &'a [u8],
}

/// Consuming iterator over String, returned from `StringArray::into_iter()`
pub struct OwnedStringIter {
    inner: Vec<u8>,
    cursor: usize,
}

impl Iterator for OwnedStringIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self
            .inner
            .iter()
            .skip(self.cursor)
            .position(|b| *b == b'\0')
        {
            None => None,
            Some(pos) => {
                let idx = self.cursor + pos;
                let ret = &self.inner[self.cursor..idx];
                self.cursor = idx + 1;
                Some(String::from_utf8_lossy(ret).to_string())
            }
        }
    }
}

/// Iterator over `CStrings` returned from `StringArray::iter_cstr()`
pub struct CStringIter<'a> {
    inner: &'a [u8],
}

impl<'a> StringArray {
    /// Create an empty `StringArray`
    #[must_use]
    pub fn empty() -> StringArray {
        StringArray(vec![])
    }
    /// Return an iterator over &str
    #[must_use]
    pub fn iter_str(&'a self) -> StringIter<'a> {
        StringIter { inner: &self.0 }
    }

    /// Return an iterator over &`CStr`
    #[must_use]
    pub fn iter_cstr(&'a self) -> CStringIter<'a> {
        CStringIter { inner: &self.0 }
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Reference to underlying bytes
    #[inline]
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<StringArray> for Vec<String> {
    fn from(a: StringArray) -> Self {
        a.into_iter().collect()
    }
}

impl<'a> Iterator for StringIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.iter().position(|c| *c == b'\0') {
            None => None,
            Some(idx) => {
                let ret = &self.inner[..idx];
                self.inner = &self.inner[idx + 1..];
                unsafe { Some(std::str::from_utf8_unchecked(ret)) }
            }
        }
    }
}

impl<'a> Iterator for CStringIter<'a> {
    type Item = &'a CStr;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.iter().position(|c| *c == b'\0') {
            None => None,
            Some(idx) => {
                let ret = &self.inner[..=idx];
                self.inner = &self.inner[idx + 1..];
                unsafe { Some(CStr::from_bytes_with_nul_unchecked(ret)) }
            }
        }
    }
}

impl IntoIterator for StringArray {
    type Item = String;
    type IntoIter = OwnedStringIter;

    fn into_iter(self) -> Self::IntoIter {
        OwnedStringIter {
            inner: self.0,
            cursor: 0,
        }
    }
}

pub(crate) fn get_string(handle: StringHandle, session: &Session) -> Result<String> {
    let bytes = crate::ffi::get_string_bytes(session, handle)?;
    String::from_utf8(bytes).map_err(crate::HapiError::from)
}

pub(crate) fn get_cstring(handle: StringHandle, session: &Session) -> Result<CString> {
    let bytes = crate::ffi::get_string_bytes(session, handle)?;
    CString::new(bytes).map_err(crate::HapiError::from)
}

pub fn get_string_array(handles: &[StringHandle], session: &Session) -> Result<StringArray> {
    let _lock = session.lock();
    let length = crate::ffi::get_string_batch_size(handles, session)?;
    let bytes = if length > 0 {
        crate::ffi::get_string_batch_bytes(length, session)?
    } else {
        vec![]
    };
    Ok(StringArray(bytes))
}

#[cfg(test)]
mod tests {
    use super::StringArray;

    #[test]
    fn string_array_empty_and_bytes() {
        let arr = StringArray::empty();
        assert!(arr.is_empty());
        assert!(arr.bytes().is_empty());
    }

    #[test]
    fn string_array_debug() {
        let arr = StringArray(b"One\0Two\0".to_vec());
        assert_eq!(format!("{arr:?}"), "StringArray[num_strings = 2]");
        let alt = format!("{arr:#?}");
        assert!(alt.contains("One") && alt.contains("Two"));
    }

    #[test]
    fn string_array_into_vec_string() {
        let arr = StringArray(b"One\0Two\0Three\0".to_vec());
        let v: Vec<String> = arr.into();
        assert_eq!(v, vec!["One", "Two", "Three"]);
    }

    #[test]
    fn string_array_iter_cstr() {
        let arr = StringArray(b"One\0Two\0Three\0".to_vec());
        let v: Vec<_> = arr.iter_cstr().collect();
        assert_eq!(v[0].to_bytes_with_nul(), b"One\0");
        assert_eq!(v[2].to_bytes_with_nul(), b"Three\0");
    }
}
