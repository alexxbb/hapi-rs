use crate::ffi;

pub type Result<T> = std::result::Result<T, HAPI_Error>;


impl ffi::HAPI_Result {
    fn description(&self) -> &str {
        use ffi::HAPI_Result::*;

        match *self {
            HAPI_RESULT_SUCCESS => "SUCCESS",
            HAPI_RESULT_FAILURE => "FAILURE",
            HAPI_RESULT_ALREADY_INITIALIZED => "ALREADY_INITIALIZED",
            HAPI_RESULT_NOT_INITIALIZED => "NOT_INITIALIZED",
            HAPI_RESULT_CANT_LOADFILE => "CANT_LOADFILE",
            HAPI_RESULT_PARM_SET_FAILED => "PARM_SET_FAILED",
            HAPI_RESULT_INVALID_ARGUMENT => "PARM_SET_FAILED",
            HAPI_RESULT_CANT_LOAD_GEO => "CANT_LOAD_GEO",
            HAPI_RESULT_CANT_GENERATE_PRESET => "CANT_GENERATE_PRESET",
            HAPI_RESULT_CANT_LOAD_PRESET => "CANT_LOAD_PRESET",
            HAPI_RESULT_ASSET_DEF_ALREADY_LOADED => "ASSET_DEF_ALREADY_LOADED",
            HAPI_RESULT_NO_LICENSE_FOUND => "NO_LICENSE_FOUND",
            HAPI_RESULT_DISALLOWED_NC_LICENSE_FOUND => "DISALLOWED_NC_LICENSE_FOUND",
            HAPI_RESULT_DISALLOWED_NC_ASSET_WITH_C_LICENSE => "DISALLOWED_NC_ASSET_WITH_C_LICENSE",
            HAPI_RESULT_DISALLOWED_NC_ASSET_WITH_LC_LICENSE => "DISALLOWED_NC_ASSET_WITH_LC_LICENSE",
            HAPI_RESULT_DISALLOWED_LC_ASSET_WITH_C_LICENSE => "DISALLOWED_LC_ASSET_WITH_C_LICENSE",
            HAPI_RESULT_DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN => "DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN",
            HAPI_RESULT_ASSET_INVALID => "ASSET_INVALID",
            HAPI_RESULT_NODE_INVALID => "NODE_INVALID",
            HAPI_RESULT_USER_INTERRUPTED => "USER_INTERRUPTED",
            HAPI_RESULT_INVALID_SESSION => "INVALID_SESSION",
        }
    }
}

#[derive(Debug)]
pub struct HAPI_Error {
    pub kind: ffi::HAPI_Result,
    session: *const ffi::HAPI_Session,
}

impl HAPI_Error {
    pub fn new(kind: ffi::HAPI_Result, session: *const ffi::HAPI_Session) -> HAPI_Error {
        HAPI_Error { kind, session }
    }
}


impl std::fmt::Display for HAPI_Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let last_error = get_last_error(self.session).expect("Could not retrieve last error");
        write!(f, "{}: {}", self.kind.description(), last_error)
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
                    session, HAPI_STATUS_CALL_RESULT,
                    // SAFETY: casting to u8 to i8 (char)?
                    buf.as_mut_ptr() as *mut i8, length) {
                    ffi::HAPI_Result::HAPI_RESULT_SUCCESS =>
                        {
                            let cs = std::ffi::CStr::from_bytes_with_nul_unchecked(&buf);
                            Ok(cs.to_str().unwrap().to_owned())
                        },
                    e => Err(HAPI_Error::new(e, session))
                }
            }
            e => Err(HAPI_Error::new(e, session))
        }
    }
}

#[macro_export]
macro_rules! ok_result {
    ($hapi_result:ident, $session:expr) => {
        match $hapi_result {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(()),
            e => Err(HAPI_Error::new(e, $session))
        }
    };
}

impl std::error::Error for HAPI_Error {}