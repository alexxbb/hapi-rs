use crate::session::Session;

pub use crate::ffi::raw::{HapiResult, StatusType, StatusVerbosity};
use std::borrow::Cow;

pub type Result<T> = std::result::Result<T, HapiError>;

/// Error type returned by all APIs
pub struct HapiError {
    /// A specific error type.
    pub kind: Kind,
    /// Context error messages.
    pub contexts: Vec<Cow<'static, str>>,
    /// Error message retrieved from the server. Server doesn't always return a message.
    pub server_message: Option<String>,
}

pub(crate) trait ErrorContext<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Into<Cow<'static, str>>;

    fn with_context<C, F>(self, func: F) -> Result<T>
    where
        C: Into<Cow<'static, str>>,
        F: FnOnce() -> C;
}

impl<T> ErrorContext<T> for Result<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Into<Cow<'static, str>>,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(mut error) => {
                error.contexts.push(context.into());
                Err(error)
            }
        }
    }

    fn with_context<C, F>(self, func: F) -> Result<T>
    where
        C: Into<Cow<'static, str>>,
        F: FnOnce() -> C,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(mut error) => {
                error.contexts.push(func().into());
                Err(error)
            }
        }
    }
}

impl PartialEq for HapiError {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind)
    }
}

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    /// Error returned by ffi calls
    Hapi(HapiResult),
    /// CString contains null byte
    NullByte,
    /// String is not a valid utf-8
    Utf8Error,
    /// Any other error
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
            contexts: Vec::new(),
            server_message: None,
        }
    }
}

impl HapiError {
    pub(crate) fn new(
        kind: Kind,
        mut context_message: Option<Cow<'static, str>>,
        server_message: Option<String>,
    ) -> HapiError {
        let mut contexts = vec![];
        if let Some(m) = context_message.take() {
            contexts.push(m);
        }
        HapiError {
            kind,
            contexts,
            server_message,
        }
    }
}

impl std::fmt::Display for HapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            Kind::Hapi(_) => {
                write!(f, "[{}]: ", self.kind.description())?;
                if let Some(ref msg) = self.server_message {
                    write!(f, "[Engine Message]: {}", msg)?;
                }
                if !self.contexts.is_empty() {
                    writeln!(f)?;
                }
                for (n, msg) in self.contexts.iter().enumerate() {
                    writeln!(f, "\t{}. {}", n, msg)?;
                }
                Ok(())
            }
            e => unreachable!("Unhandled error kind: {:?}", &e),
        }
    }
}

impl std::fmt::Debug for HapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        context_msg: Option<Cow<'static, str>>,
    ) -> Result<R> {
        match self {
            HapiResult::Success => Ok(R::default()),
            err => {
                let mut server_msg = None;
                if let Some(session) = session {
                    let err_msg = session
                        .get_status_string(StatusType::CallResult, StatusVerbosity::All)
                        .unwrap_or_else(|_| String::from("could not retrieve error message"));
                    server_msg.replace(err_msg);
                }
                Err(HapiError::new(Kind::Hapi(err), context_msg, server_msg))
            }
        }
    }

    #[inline]
    pub(crate) fn check_err<R: Default>(self, session: Option<&Session>) -> Result<R> {
        self._into_result(session, None)
    }

    #[inline]
    pub(crate) fn error_message<I: Into<Cow<'static, str>>, R: Default>(
        self,
        message: I,
    ) -> Result<R> {
        self._into_result(None, Some(message.into()))
    }
}
