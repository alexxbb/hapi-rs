//! String handling utilities and iterators
use std::ffi::{CStr, CString};
use std::fmt::Formatter;

use crate::errors::{ErrorContext, Result};
use crate::session::Session;

// StringArray iterators SAFETY: Are Houdini strings expected to be valid utf? Maybe revisit.

/// A handle to a string returned by some api.
/// Then the String can be retrieved with [`Session::get_string`]
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct StringHandle(pub(crate) i32);

/// Holds a contiguous array of bytes where each individual string value is null-separated.
/// You can choose how to iterate over it by calling a corresponding iter_* function.
/// The `Debug` impl has an alternative `{:#?}` representation, which prints as a vec of strings.
#[derive(Clone)]
pub struct StringArray(Vec<u8>);

impl std::fmt::Debug for StringArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            let strings = self.iter_str().collect::<Vec<_>>();
            strings.fmt(f)
        } else {
            let count = self.0.iter().filter(|v| **v == b'\0').count();
            write!(f, "StringArray[num_strings = {count}]")
        }
    }
}

/// Iterator over &str, returned from StringArray::iter_str()
pub struct StringIter<'a> {
    inner: &'a [u8],
}

/// Consuming iterator over String, returned from StringArray::into_iter()
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

/// Iterator over CStrings returned from StringArray::iter_cstr()
pub struct CStringIter<'a> {
    inner: &'a [u8],
}

impl<'a> StringArray {
    /// Create an empty StringArray
    pub fn empty() -> StringArray {
        StringArray(vec![])
    }
    /// Return an iterator over &str
    pub fn iter_str(&'a self) -> StringIter<'a> {
        StringIter { inner: &self.0 }
    }

    /// Return an iterator over &CStr
    pub fn iter_cstr(&'a self) -> CStringIter<'a> {
        CStringIter { inner: &self.0 }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Reference to underlying bytes
    #[inline]
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
                let ret = &self.inner[..idx + 1];
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
    let bytes = get_string_bytes(handle, session).context("Calling get_string_bytes")?;
    String::from_utf8(bytes).map_err(crate::errors::HapiError::from)
}

pub(crate) fn get_cstring(handle: StringHandle, session: &Session) -> Result<CString> {
    unsafe {
        let bytes = get_string_bytes(handle, session).context("Calling get_string_bytes")?;
        // SAFETY: HAPI C API should not return strings with interior zero byte
        Ok(CString::from_vec_unchecked(bytes))
    }
}

pub(crate) fn get_string_bytes(handle: StringHandle, session: &Session) -> Result<Vec<u8>> {
    if handle.0 < 0 {
        return Ok(Vec::new());
    }
    let length = crate::ffi::get_string_buff_len(session, handle.0)?;
    if length == 0 {
        Ok(Vec::new())
    } else {
        crate::ffi::get_string(session, handle.0, length)
    }
}

pub fn get_string_array(handles: &[StringHandle], session: &Session) -> Result<StringArray> {
    let _lock = session.lock();
    let length = crate::ffi::get_string_batch_size(handles, session)?;
    let bytes = if length > 0 {
        crate::ffi::get_string_batch(length, session)?
    } else {
        vec![]
    };
    Ok(StringArray(bytes))
}

#[cfg(test)]
mod tests {
    use super::StringArray;
    use crate::ffi;
    use crate::session::Session;
    use once_cell::sync::Lazy;
    use std::ffi::CString;

    static SESSION: Lazy<Session> = Lazy::new(|| {
        let _ = env_logger::try_init().ok();
        crate::session::quick_session(None, None).expect("Could not create test session")
    });

    #[test]
    fn get_string_api() {
        let h = ffi::get_server_env_str(&SESSION, &CString::new("HFS").unwrap()).unwrap();
        assert!(super::get_string(h, &SESSION).is_ok());
        assert!(super::get_cstring(h, &SESSION).is_ok());
    }

    #[test]
    fn string_array_api() {
        SESSION
            .set_server_var::<str>("TEST", "177")
            .expect("could not set var");
        let var_count = ffi::get_server_env_var_count(&SESSION).unwrap();
        let handles = ffi::get_server_env_var_list(&SESSION, var_count).unwrap();
        let array = super::get_string_array(&handles, &SESSION).unwrap();
        assert_eq!(array.iter_str().count(), var_count as usize);
        assert_eq!(array.iter_cstr().count(), var_count as usize);
        assert!(array.iter_str().any(|s| s == "TEST=177"));
        assert!(
            array
                .iter_cstr()
                .any(|s| s.to_bytes_with_nul() == b"TEST=177\0")
        );
        let mut owned: super::OwnedStringIter = array.into_iter();
        assert!(owned.any(|s| s == "TEST=177"));

        let arr = StringArray(b"One\0Two\0Three\0".to_vec());
        let v: Vec<_> = arr.iter_cstr().collect();
        assert_eq!(v[0].to_bytes_with_nul(), b"One\0");
        assert_eq!(v[2].to_bytes_with_nul(), b"Three\0");
    }
}
