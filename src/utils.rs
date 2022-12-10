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
