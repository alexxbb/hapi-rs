use crate::{auto::bindings as ffi, check_session, session::Session};
use std::cell::Cell;

// TODO: Rethink the design. Passing raw pointer to session may be not a good idea
pub type Result<T> = std::result::Result<T, HapiError>;

#[derive(Debug)]
pub struct HapiError {
    pub kind: Kind,
    pub message: Option<&'static str>,
    pub(crate) session: Option<Session>,
}

#[derive(Debug)]
pub enum Kind {
    Hapi(ffi::HAPI_Result),
    CookError,
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
            Kind::CookError => "Cooking error",
            Kind::Hapi(_) => unreachable!(),
        }
    }
}

impl HapiError {
    pub(crate) fn new(
        kind: Kind,
        session: Option<Session>,
        message: Option<&'static str>,
    ) -> HapiError {
        HapiError {
            kind,
            session,
            message,
        }
    }
}

impl std::fmt::Display for HapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            Kind::Hapi(_) => {
                if let Some(session) = &self.session {
                    check_session!(session.ptr());
                    let error = get_last_error(&session);
                    write!(
                        f,
                        "{}: {}",
                        self.kind.description(),
                        error
                            .ok()
                            .or_else(||self.message.map(|s|s.to_string()))
                            .unwrap_or_else(||String::from("Zig"))
                    )
                } else {
                    write!(f, "{}", self.kind.description())
                }
            }
            _ => unreachable!(),
        }
    }
}

// TODO cooking errors
pub fn get_last_error(session: &Session) -> Result<String> {
    use ffi::HAPI_StatusType::HAPI_STATUS_CALL_RESULT;
    use ffi::HAPI_StatusVerbosity::HAPI_STATUSVERBOSITY_0;
    unsafe {
        let mut length = std::mem::MaybeUninit::uninit();
        ffi::HAPI_GetStatusStringBufLength(
            session.ptr(),
            HAPI_STATUS_CALL_RESULT,
            HAPI_STATUSVERBOSITY_0,
            length.as_mut_ptr(),
        )
        .with_message(Some("GetStatusStringBufLength failed"))?;
        let length = length.assume_init();
        let mut buf = vec![0u8; length as usize];
        ffi::HAPI_GetStatusString(
            session.ptr(),
            HAPI_STATUS_CALL_RESULT,
            // SAFETY: casting to u8 to i8 (char)?
            buf.as_mut_ptr() as *mut i8,
            length,
        )
        .with_message(Some("GetStatusString failed"))?;
        buf.truncate(length as usize);
        Ok(String::from_utf8_unchecked(buf))
    }
}

impl From<std::ffi::NulError> for HapiError {
    fn from(_: std::ffi::NulError) -> Self {
        HapiError::new(Kind::NullByte, None, None)
    }
}

#[macro_export]
macro_rules! hapi_ok {
    ($hapi_result:expr, $session:expr, $message:expr) => {
        match $hapi_result {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(()),
            e => Err(HapiError::new(Kind::Hapi(e), $session, $message)),
        }
    };
}

#[macro_export]
macro_rules! hapi_err {
    ($hapi_result:expr, $session:expr, $message:expr) => {
        Err(HapiError::new(
            Kind::Hapi($hapi_result),
            Some($session),
            $expr,
        ))
    };

    ($hapi_result:expr) => {
        Err(HapiError::new(Kind::Hapi($hapi_result), None, None))
    };
}

impl std::error::Error for HapiError {}

impl ffi::HAPI_Result {
    pub(crate) fn err<F>(self, op: F) -> Result<()>
    where
        F: FnOnce() -> (Option<Session>, Option<&'static str>),
    {
        match self {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(()),
            e => {
                let (session, message) = op();
                Err(HapiError::new(Kind::Hapi(e), session, message))
            }
        }
    }
    pub(crate) fn with_session<F>(self, op: F) -> Result<()>
        where
            F: FnOnce() -> Session,
    {
        self.err(||(Some(op()), None))
    }

    pub(crate) fn with_message(self, msg: Option<&'static str>) -> Result<()> {
        self.err(||(None, msg))
    }
}
