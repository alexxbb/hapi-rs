use crate::ffi;
use std::mem::MaybeUninit;
use crate::errors::HAPI_Error;
use std::ptr::null;
use crate::cookoptions::CookOptions;

pub type Result<T> = std::result::Result<T, HAPI_Error>;

use std::path::Path;
use std::ffi::{CString, CStr};

fn join_paths<'a, I>(files: I) -> String
    where I: IntoIterator,
          I::Item: AsRef<Path>
{
    let mut buf = String::new();
    let mut iter = files.into_iter().peekable();
    while let Some(n) = iter.next() {
        buf.push_str(&n.as_ref().to_string_lossy());
        if iter.peek().is_some() {
            buf.push(':');
        }
    }
    buf
}

#[derive(Debug)]
pub struct Session {
    inner: ffi::HAPI_Session
}

pub struct Initializer<'a> {
    session: Option<&'a Session>,
    cook_opt: Option<&'a CookOptions>,
    cook_thread: bool,
    env_files: Option<CString>,
}

impl<'a> Initializer<'a> {
    pub fn new() -> Initializer<'a> {
        Initializer { session: None, cook_opt: None, cook_thread: false, env_files: None }
    }

    pub fn set_env_files<Files>(&mut self, files: Files)
        where Files: IntoIterator,
              Files::Item: AsRef<Path>
    {
        let paths = join_paths(files);
        self.env_files.replace(CString::new(paths).expect("Zero byte"));
    }

    pub fn initialize(self) -> Result<()> {
        unsafe {
            let result = ffi::HAPI_Initialize(
                self.session.map(|s| s.ptr()).unwrap_or(null()),
                self.cook_opt.map(|o| o.ptr()).unwrap_or(CookOptions::default().ptr()),
                self.cook_thread as i8,
                -1,
                self.env_files.map(|p| p.as_ptr()).unwrap_or(null()),
                null(),
                null(),
                null(),
                null(),
            );
        }

        Ok(())
    }
}


// impl<'a, Files> Initializer<'a, Files>
//     where Files: IntoIterator,
//           Files::Item: AsRef<Path>
// {
//     pub fn new() -> Initializer<'a, Files> {
//         Initializer { session: None, cook_opt: None, cook_thread: false, env_files: None }
//     }
//     pub fn with_cook_thread(&mut self, thread: bool) -> &mut Self {
//         self.cook_thread = thread;
//         self
//     }
//     pub fn with_session(&mut self, session: &'a Session) -> &mut Self {
//         self.session.replace(session);
//         self
//     }
//
//     pub fn with_cook_options(&mut self, opts: &'a CookOptions) -> &mut Self {
//         self.cook_opt.replace(opts);
//         self
//     }
//     pub fn with_env_files<Files>(&mut self, files: Files) -> &mut Self
//     where Files: IntoIterator,
//           Files::Item: AsRef<Path>
//     {
//         self.env_files.replace(files);
//         self
//     }
//
//     }
// }

impl Session {
    pub fn new_in_process() -> Result<Session> {
        let mut s = MaybeUninit::uninit();
        unsafe {
            match ffi::HAPI_CreateInProcessSession(s.as_mut_ptr()) {
                ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                    Ok(Session { inner: s.assume_init() })
                }
                e => Err(e.into())
            }
        }
    }

    #[inline]
    pub fn ptr(&self) -> *const ffi::HAPI_Session {
        &self.inner as *const ffi::HAPI_Session
    }

    #[inline]
    pub fn mut_ptr(&mut self) -> *mut ffi::HAPI_Session {
        self.ptr() as *mut ffi::HAPI_Session
    }
}

