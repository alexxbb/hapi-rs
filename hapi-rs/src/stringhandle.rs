use std::ffi::CString;

use crate::errors::{HapiError, Kind, Result};
use crate::session::Session;

#[derive(Debug)]
pub struct StringBuffer {
    bytes: Vec<u8>,
}

pub struct StringIter<'a> {
    inner: &'a [u8],
}

impl<'a> StringBuffer {
    pub fn iter_str(&'a self) -> StringIter<'a> {
        StringIter { inner: &self.bytes }
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

pub fn get_string(handle: i32, session: &Session) -> Result<String> {
    unsafe {
        let mut bytes = get_string_bytes(handle, session)?;
        Ok(String::from_utf8_unchecked(bytes))
    }
}

pub fn get_cstring(handle: i32, session: &Session) -> Result<CString> {
    unsafe {
        let mut bytes = get_string_bytes(handle, session)?;
        Ok(CString::from_vec_unchecked(bytes))
    }
}

pub(crate) fn get_string_bytes(handle: i32, session: &Session) -> Result<Vec<u8>> {
    unsafe {
        let length = crate::ffi::get_string_buff_len(session, handle)?;
        let buffer = crate::ffi::get_string(session, handle, length)?;
        Ok(buffer)
    }
}

pub fn get_string_array_bytes(handles: &[i32], session: &Session) -> Result<Vec<u8>> {
    unsafe {
        let length = crate::ffi::get_string_batch_size(handles, session)?;
        if length > 0 {
            Ok(crate::ffi::get_string_batch(length, session)?)
        } else {
            Ok(vec![])
        }
    }
}

pub fn get_string_batch(handles: &[i32], session: &Session) -> Result<Vec<String>> {
    let buffer = get_string_array_bytes(handles, session)?;
    unsafe {
        if buffer.len() > 0 {
            let mut buffer = std::mem::ManuallyDrop::new(buffer);
            let strings = buffer
                .split_mut(|b| *b == b'\0')
                .map(|s| String::from_utf8_lossy(s).to_string())
                .collect::<Vec<String>>();
            Ok(strings)
        } else {
            Ok(vec![])
        }
    }
}

#[inline]
pub fn get_string_buffer(handles: &[i32], session: &Session) -> Result<StringBuffer> {
    Ok(StringBuffer {
        bytes: get_string_array_bytes(handles, session)?,
    })
}
