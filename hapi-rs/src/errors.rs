use crate::session::Session;

pub use crate::ffi::raw::{HapiResult, StatusType, StatusVerbosity};
use std::borrow::Cow;
use std::fmt::Formatter;

pub type Result<T> = std::result::Result<T, HapiError>;

pub struct HapiError {
    pub kind: Kind,
    pub message: Option<Cow<'static, str>>,
    pub(crate) session: Option<Session>,
}

impl PartialEq for HapiError {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind)
    }
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Kind {
    Hapi(HapiResult),
    NullByte,
    Utf8Error,
    Other(String),
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
            Kind::Hapi(InvalidArgument) => "INVALID_ARGUMENT",
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
            Kind::Utf8Error => "String is not UTF-8!",
            Kind::Other(s) => s,
        }
    }
}

impl From<HapiResult> for HapiError {
    fn from(r: HapiResult) -> Self {
        HapiError {
            kind: Kind::Hapi(r),
            message: None,
            session: None,
        }
    }
}

impl HapiError {
    pub(crate) fn new(
        kind: Kind,
        session: Option<Session>,
        message: Option<Cow<'static, str>>,
    ) -> HapiError {
        HapiError {
            kind,
            message,
            session,
        }
    }
}

impl std::fmt::Display for HapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            Kind::Hapi(_) => {
                if let Some(ref session) = self.session {
                    let err_msg = session
                        .get_status_string(StatusType::CallResult, StatusVerbosity::Errors)
                        .unwrap_or_else(|_| String::from("could not retrieve error message"));
                    write!(f, "[{}]: ", self.kind.description())?;
                    write!(f, "[Engine Message]: {} ", err_msg)?;
                }
                if let Some(ref msg) = self.message {
                    write!(f, "[Context Message]: {}", msg)?;
                }
                Ok(())
            }
            e => unreachable!("Unhandled error kind: {:?}", &e),
        }
    }
}

impl std::fmt::Debug for HapiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<std::ffi::NulError> for HapiError {
    fn from(_: std::ffi::NulError) -> Self {
        HapiError::new(Kind::NullByte, None, None)
    }
}

impl From<std::string::FromUtf8Error> for HapiError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        HapiError::new(Kind::Utf8Error, None, None)
    }
}

impl std::error::Error for HapiError {}

impl HapiResult {
    fn _into_result<R: Default>(
        self,
        session: Option<&Session>,
        message: Option<Cow<'static, str>>,
    ) -> Result<R> {
        match self {
            HapiResult::Success => Ok(R::default()),
            err => Err(HapiError::new(Kind::Hapi(err), session.cloned(), message)),
        }
    }

    pub(crate) fn check_err<R: Default>(self, session: Option<&Session>) -> Result<R> {
        self._into_result(session, None)
    }

    pub(crate) fn error_message<I: Into<Cow<'static, str>>, R: Default>(
        self,
        message: I,
    ) -> Result<R> {
        self._into_result(None, Some(message.into()))
    }
}
