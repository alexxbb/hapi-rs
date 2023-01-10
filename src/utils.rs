#[allow(unused)]
macro_rules! cstr {
    ($($bytes:tt)*) => {
        CStr::from_bytes_with_nul($($bytes)*).expect("CStr with null")
    };
}

macro_rules! unwrap_or_create {
    ($out:ident, $opt:expr, $default:expr) => {
        match $opt {
            None => {
                $out = $default;
                &$out
            }
            Some(v) => v,
        }
    };
}

#[allow(unused)]
pub(crate) use cstr;
pub(crate) use unwrap_or_create;

pub(crate) fn path_to_cstring(
    path: impl AsRef<std::path::Path>,
) -> crate::Result<std::ffi::CString> {
    let s = path.as_ref().as_os_str().to_string_lossy().to_string();
    Ok(std::ffi::CString::new(s)?)
}

/// Join a sequence of paths into a single String
pub fn join_paths<I>(files: I) -> String
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut buf = String::new();
    let mut iter = files.into_iter().peekable();
    while let Some(n) = iter.next() {
        buf.push_str(n.as_ref());
        if iter.peek().is_some() {
            buf.push(':');
        }
    }
    buf
}
