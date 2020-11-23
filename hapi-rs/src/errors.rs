pub use crate::auto::rusty::HapiResult;
use crate::auto::rusty::{StatusType, StatusVerbosity};
use crate::{auto::bindings as ffi, check_session, session::Session};

pub type Result<T> = std::result::Result<T, HapiError>;

#[derive(Debug)]
pub struct HapiError {
    pub kind: Kind,
    pub message: Option<&'static str>,
    pub(crate) session: Option<Session>,
}

#[derive(Debug)]
pub enum Kind {
    Hapi(HapiResult),
    CookError,
    NullByte,
}

impl Kind {
    fn description(&self) -> &str {
        use HapiResult::*;

        match self {
            Kind::Hapi(Success) => "SUCCESS",
            Kind::Hapi(Failure) => "FAILURE",
            Kind::Hapi(AlreadyInitialized) => "ALREADY_INITIALIZED",
            Kind::Hapi(NotInitialized) => "NOT_INITIALIZED",
            Kind::Hapi(CantLoadfile) => "CANT_LOADFILE",
            Kind::Hapi(ParmSetFailed) => "PARM_SET_FAILED",
            Kind::Hapi(InvalidArgument) => "PARM_SET_FAILED",
            Kind::Hapi(CantLoadGeo) => "CANT_LOAD_GEO",
            Kind::Hapi(CantGeneratePreset) => "CANT_GENERATE_PRESET",
            Kind::Hapi(CantLoadPreset) => "CANT_LOAD_PRESET",
            Kind::Hapi(AssetDefAlreadyLoaded) => "ASSET_DEF_ALREADY_LOADED",
            Kind::Hapi(NoLicenseFound) => "NO_LICENSE_FOUND",
            Kind::Hapi(DisallowedNcLicenseFound) => "DISALLOWED_NC_LICENSE_FOUND",
            Kind::Hapi(DisallowedNcAssetWithCLicense) => "DISALLOWED_NC_ASSET_WITH_C_LICENSE",
            Kind::Hapi(DisallowedNcAssetWithLcLicense) => "DISALLOWED_NC_ASSET_WITH_LC_LICENSE",
            Kind::Hapi(DisallowedLcAssetWithCLicense) => "DISALLOWED_LC_ASSET_WITH_C_LICENSE",
            Kind::Hapi(DisallowedHengineindieW3partyPlugin) => {
                "DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN"
            }
            Kind::Hapi(AssetInvalid) => "ASSET_INVALID",
            Kind::Hapi(NodeInvalid) => "NODE_INVALID",
            Kind::Hapi(UserInterrupted) => "USER_INTERRUPTED",
            Kind::Hapi(InvalidSession) => "INVALID_SESSION",
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
                    let error = get_call_status(&session);
                    write!(
                        f,
                        "{}: {}",
                        self.kind.description(),
                        error
                            .ok()
                            .or_else(|| self.message.map(|s| s.to_string()))
                            .unwrap_or_else(|| String::from("Zig"))
                    )
                } else if let Some(msg) = self.message {
                    write!(f, "Kind:{}, Message: {}", self.kind.description(), msg)
                } else {
                    write!(f, "Kind:{}", self.kind.description())
                }
            }
            _ => unreachable!(),
        }
    }
}

#[inline]
pub fn get_call_status(session: &Session) -> Result<String> {
    get_status_string(
        session,
        StatusType::CallResult,
        StatusVerbosity::Statusverbosity0,
    )
}

pub fn get_cook_status(session: &Session) -> Result<String> {
    get_status_string(
        session,
        StatusType::CookResult,
        StatusVerbosity::Statusverbosity0,
    )
}

fn get_status_string(
    session: &Session,
    type_: StatusType,
    verbosity: StatusVerbosity,
) -> Result<String> {
    unsafe {
        let mut length = std::mem::MaybeUninit::uninit();
        ffi::HAPI_GetStatusStringBufLength(
            session.ptr(),
            type_.into(),
            verbosity.into(),
            length.as_mut_ptr(),
        )
        .result_with_message(Some("GetStatusStringBufLength failed"))?;
        let length = length.assume_init();
        let mut buf = vec![0u8; length as usize];
        if length > 0 {
            ffi::HAPI_GetStatusString(
                session.ptr(),
                type_.into(),
                // SAFETY: casting to u8 to i8 (char)?
                buf.as_mut_ptr() as *mut i8,
                length,
            )
            .result_with_message(Some("GetStatusString failed"))?;
            buf.truncate(length as usize);
            Ok(String::from_utf8_unchecked(buf))
        } else {
            Ok(String::new())
        }
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
            e => Err(HapiError::new(Kind::Hapi(e.into()), $session, $message)),
        }
    };
}

#[macro_export]
macro_rules! hapi_err {
    ($hapi_result:expr, $session:expr, $message:expr) => {
        Err(HapiError::new(
            Kind::Hapi($hapi_result.into()),
            $session,
            $message,
        ))
    };

    ($hapi_result:expr) => {
        Err(HapiError::new(Kind::Hapi($hapi_result.into()), None, None))
    };
}

impl std::error::Error for HapiError {}

impl ffi::HAPI_Result {
    pub(crate) fn result<R, F>(self, op: F, ret: R) -> Result<R>
    where
        F: FnOnce() -> (Option<Session>, Option<&'static str>),
    {
        match self {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => Ok(ret),
            e => {
                let (session, message) = op();
                Err(HapiError::new(Kind::Hapi(e.into()), session, message))
            }
        }
    }
    pub(crate) fn result_with_session<F>(self, op: F) -> Result<()>
    where
        F: FnOnce() -> Session,
    {
        self.result(|| (Some(op()), None), ())
    }

    pub(crate) fn result_with_message(self, msg: Option<&'static str>) -> Result<()> {
        self.result(|| (None, msg), ())
    }
}
