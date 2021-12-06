use std::ffi::{CStr, CString};

use crate::errors::Result;
use crate::session::Session;

// StringArray iterators SAFETY: Are Houdini strings expected to be valid utf? Maybe revisit.

#[derive(Debug)]
pub struct StringArray {
    bytes: Vec<u8>,
}

pub struct StringIter<'a> {
    inner: &'a [u8],
}

pub struct OwnedStringIter {
    inner: Vec<u8>,
    cursor: usize,
}

impl std::iter::Iterator for OwnedStringIter {
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

pub struct CStringIter<'a> {
    inner: &'a [u8],
}

impl<'a> StringArray {
    pub fn iter_str(&'a self) -> StringIter<'a> {
        StringIter { inner: &self.bytes }
    }

    pub fn iter_cstr(&'a self) -> CStringIter<'a> {
        CStringIter { inner: &self.bytes }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl<'a> std::iter::Iterator for StringIter<'a> {
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

impl<'a> std::iter::Iterator for CStringIter<'a> {
    type Item = &'a std::ffi::CStr;

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

impl std::iter::IntoIterator for StringArray {
    type Item = String;
    type IntoIter = OwnedStringIter;

    fn into_iter(self) -> Self::IntoIter {
        OwnedStringIter {
            inner: self.bytes,
            cursor: 0,
        }
    }
}

pub fn get_string(handle: i32, session: &Session) -> Result<String> {
    let bytes = get_string_bytes(handle, session)?;
    String::from_utf8(bytes).map_err(crate::errors::HapiError::from)
}

pub fn get_cstring(handle: i32, session: &Session) -> Result<CString> {
    unsafe {
        let bytes = get_string_bytes(handle, session)?;
        // SAFETY: HAPI C API should not return strings with interior zero byte
        Ok(CString::from_vec_unchecked(bytes))
    }
}

pub fn get_string_bytes(handle: i32, session: &Session) -> Result<Vec<u8>> {
    let length = crate::ffi::get_string_buff_len(session, handle)?;
    let buffer = crate::ffi::get_string(session, handle, length)?;
    Ok(buffer)
}

pub fn get_string_array(handles: &[i32], session: &Session) -> Result<StringArray> {
    let _lock = session.handle.1.lock();
    let length = crate::ffi::get_string_batch_size(handles, session)?;
    let bytes = if length > 0 {
        crate::ffi::get_string_batch(length, session)?
    } else {
        vec![]
    };
    Ok(StringArray { bytes })
}

#[cfg(test)]
mod tests {
    use crate::ffi;
    use crate::session::simple_session;
    use crate::session::tests::with_session;
    use std::ffi::{CStr, CString};

    #[test]
    fn test_get_string() {
        with_session(|session| {
            let h = ffi::get_server_env_str(session, &CString::new("HFS").unwrap()).unwrap();
            assert!(super::get_string(h, session).is_ok());
            assert!(super::get_cstring(h, session).is_ok());
        });
    }

    #[test]
    fn test_string_array() {
        let session = simple_session(None).expect("simple session");
        session
            .set_server_var::<str>("TEST", "177")
            .expect("could not set var");
        let var_count = ffi::get_server_env_var_count(&session).unwrap();
        let handles = ffi::get_server_env_var_list(&session, var_count).unwrap();
        let array = super::get_string_array(&handles, &session).unwrap();
        assert_eq!(array.iter_str().count(), var_count as usize);
        assert_eq!(array.iter_cstr().count(), var_count as usize);
        assert!(array.iter_str().any(|s| s == "TEST=177"));
        assert!(array
            .iter_cstr()
            .any(|s| s == unsafe { CStr::from_bytes_with_nul_unchecked(b"TEST=177\0") }));
        let mut owned: super::OwnedStringIter = array.into_iter();
        assert!(owned.any(|s| s == "TEST=177"));
    }
}
