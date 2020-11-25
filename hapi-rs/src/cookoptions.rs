use crate::auto::bindings as ffi;

pub struct CookOptions {
    inner: ffi::HAPI_CookOptions,
}

impl Default for CookOptions {
    fn default() -> CookOptions {
        CookOptions {
            inner: unsafe { ffi::HAPI_CookOptions_Create() },
        }
    }
}

impl CookOptions {
    pub fn ptr(&self) -> *const ffi::HAPI_CookOptions {
        &self.inner as *const _
    }
}
