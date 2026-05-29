#[allow(unused)]
macro_rules! cstr {
    ($($bytes:tt)*) => {
        CStr::from_bytes_with_nul($($bytes)*).expect("CStr with null")
    };
}

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
    const SEP: char = if cfg!(windows) { ';' } else { ':' };
    let mut buf = String::new();
    let mut iter = files.into_iter().peekable();
    while let Some(n) = iter.next() {
        buf.push_str(n.as_ref());
        if iter.peek().is_some() {
            buf.push(SEP);
        }
    }
    buf
}

/// Generates a random ascii a-z sequence of specified length
pub fn random_string(len: usize) -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut seed = RandomState::new().build_hasher().finish();
    let next = || {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let letter = u8::try_from(seed % 26).expect("seed modulo 26 always fits in u8");
        char::from(letter + b'a')
    };
    std::iter::repeat_with(next).take(len).collect()
}

#[inline]
pub(crate) fn uzize_to_i32(value: usize) -> i32 {
    i32::try_from(value).expect("usize->i32 overflow")
}

#[inline]
pub(crate) fn i32_to_usize(value: i32) -> usize {
    usize::try_from(value).expect("i32->usize underflow")
}

#[inline]
pub(crate) fn i64_to_usize(value: i64) -> usize {
    usize::try_from(value).expect("i64->usize underflow")
}

#[inline]
pub(crate) fn i64_to_i32_clamped(value: i64) -> i32 {
    i32::try_from(value.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap()
}

#[cfg(test)]
mod tests {
    use super::join_paths;

    const SEP: char = if cfg!(windows) { ';' } else { ':' };

    #[test]
    fn join_paths_empty() {
        assert_eq!(join_paths([] as [&str; 0]), "");
    }

    #[test]
    fn join_paths_single() {
        assert_eq!(join_paths(["/one/path"]), "/one/path");
    }

    #[test]
    fn join_paths_multiple() {
        assert_eq!(
            join_paths(["/a", "/b", "/c"]),
            format!("/a{SEP}/b{SEP}/c")
        );
    }
}
