use crate::ffi::HAPI_Session;

pub trait ConstPtr<T> {
    fn const_ptr(&self) -> *const T;
}

impl ConstPtr<HAPI_Session> for HAPI_Session {
    fn const_ptr(&self) -> *const Self {
        self as *const Self
    }
}
