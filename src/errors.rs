use crate::ffi;
use std::cell::Cell;

pub type Result<T> = std::result::Result<T, HAPI_Error>;

#[derive(Debug)]
pub struct HAPI_Error {
    pub kind: Kind,
    pub(crate) session: Option<Cell<*const ffi::HAPI_Session>>,
}

#[derive(Debug)]
pub enum Kind {
    Hapi(ffi::HAPI_Result),
    NullByte,
}

impl Kind {
    fn description(&self) -> &str {
        use ffi::HAPI_Result::*;

        match self {
            Kind::Hapi(HAPI_RESULT_SUCCESS) => "SUCCESS",
            Kind::Hapi(HAPI_RESULT_FAILURE) => "FAILURE",
            Kind::Hapi(HAPI_RESULT_ALREADY_INITIALIZED) => "ALREADY_INITIALIZED",
            Kind::Hapi(HAPI_RESULT_NOT_INITIALIZED) => "NOT_INITIALIZED",
            Kind::Hapi(HAPI_RESULT_CANT_LOADFILE) => "CANT_LOADFILE",
            Kind::Hapi(HAPI_RESULT_PARM_SET_FAILED) => "PARM_SET_FAILED",
            Kind::Hapi(HAPI_RESULT_INVALID_ARGUMENT) => "PARM_SET_FAILED",
            Kind::Hapi(HAPI_RESULT_CANT_LOAD_GEO) => "CANT_LOAD_GEO",
            Kind::Hapi(HAPI_RESULT_CANT_GENERATE_PRESET) => "CANT_GENERATE_PRESET",
            Kind::Hapi(HAPI_RESULT_CANT_LOAD_PRESET) => "CANT_LOAD_PRESET",
            Kind::Hapi(HAPI_RESULT_ASSET_DEF_ALREADY_LOADED) => "ASSET_DEF_ALREADY_LOADED",
            Kind::Hapi(HAPI_RESULT_NO_LICENSE_FOUND) => "NO_LICENSE_FOUND",
            Kind::Hapi(HAPI_RESULT_DISALLOWED_NC_LICENSE_FOUND) => "DISALLOWED_NC_LICENSE_FOUND",
            Kind::Hapi(HAPI_RESULT_DISALLOWED_NC_ASSET_WITH_C_LICENSE) => {
                "DISALLOWED_NC_ASSET_WITH_C_LICENSE"
            }
            Kind::Hapi(HAPI_RESULT_DISALLOWED_NC_ASSET_WITH_LC_LICENSE) => {
                "DISALLOWED_NC_ASSET_WITH_LC_LICENSE"
            }
            Kind::Hapi(HAPI_RESULT_DISALLOWED_LC_ASSET_WITH_C_LICENSE) => {
                "DISALLOWED_LC_ASSET_WITH_C_LICENSE"
            }
            Kind::Hapi(HAPI_RESULT_DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN) => {
                "DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN"
            }
            Kind::Hapi(HAPI_RESULT_ASSET_INVALID) => "ASSET_INVALID",
            Kind::Hapi(HAPI_RESULT_NODE_INVALID) => "NODE_INVALID",
            Kind::Hapi(HAPI_RESULT_USER_INTERRUPTED) => "USER_INTERRUPTED",
            Kind::Hapi(HAPI_RESULT_INVALID_SESSION) => "INVALID_SESSION",
            Kind::NullByte => "String contains null byte!",
        }
    }
}

impl HAPI_Error {
    pub fn new(kind: Kind, session: Option<*const ffi::HAPI_Session>) -> HAPI_Error {
        HAPI_Error {
            kind,
            session: session.map(|s| Cell::new(s)),
        }
    }
}

impl std::fmt::Display for HAPI_Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            Kind::Hapi(_) => {
                if let Some(session) = &self.session {
                    let last_error =
                        get_last_error(session.get()).expect("Could not retrieve last error");
                    write!(f, "{}: {}", self.kind.description(), last_error)
                } else {
                    write!(f, "{}", self.kind.description())
                }
            }
            _ => unreachable!(),
        }
    }
}

pub fn get_last_error(session: *const ffi::HAPI_Session) -> Result<String> {
    use ffi::HAPI_StatusType::HAPI_STATUS_CALL_RESULT;
    use ffi::HAPI_StatusVerbosity::HAPI_STATUSVERBOSITY_0;
    unsafe {
        let mut length = std::mem::MaybeUninit::uninit();
        let res = ffi::HAPI_GetStatusStringBufLength(
            session,
            HAPI_STATUS_CALL_RESULT,
            HAPI_STATUSVERBOSITY_0,
            length.as_mut_ptr(),
        );
        match res {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                let length = length.assume_init();
                let mut buf = vec![0u8; length as usize];
                match ffi::HAPI_GetStatusString(
                    session,
                    HAPI_STATUS_CALL_RESULT,
                    // SAFETY: casting to u8 to i8 (char)?
                    buf.as_mut_ptr() as *mut i8,
                    length,
                ) {
                    ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                        let cs = std::ffi::CStr::from_bytes_with_nul_unchecked(&buf);
                        Ok(cs.to_str().unwrap().to_owned())
                    }
                    e => Err(HAPI_Error::new(Kind::Hapi(e), Some(session))),
                }
            }
            e => Err(HAPI_Error::new(Kind::Hapi(e), Some(session))),
        }
    }
}

impl From<std::ffi::NulError> for HAPI_Error {
    fn from(_: std::ffi::NulError) -> Self {
        HAPI_Error::new(Kind::NullByte, None)
    }
}

#[macro_export]
macro_rules! ok_result {
    ($hapi_result:expr, $session:expr) => {
        match $hapi_result {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(()),
            e => Err(HAPI_Error::new(Kind::Hapi(e), Some($session))),
        }
    };
}

impl std::error::Error for HAPI_Error {}
