use crate::{node::NodeHandle, session::Session};

macro_rules! char_ptr {
    ($lit:expr) => {{
        use std::ffi::CStr;
        use std::os::raw::c_char;
        unsafe { CStr::from_ptr(concat!($lit, "\0").as_ptr() as *const c_char).as_ptr() }
    }};
}

macro_rules! check_session {
    ($session:expr) => {
        use crate::ffi::{HAPI_IsSessionValid, HapiResult};
        assert!(
            unsafe { matches!(HAPI_IsSessionValid($session), HapiResult::Success) },
            "Session is invalid!"
        );
    };
}
