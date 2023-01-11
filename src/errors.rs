use crate::session::Session;

pub use crate::ffi::raw::{HapiResult, StatusType, StatusVerbosity};
use std::borrow::Cow;

pub type Result<T> = std::result::Result<T, HapiError>;

/// Error type returned by all APIs
// TODO: This should really be an Enum.
pub struct HapiError {
    /// A specific error type.
    pub kind: Kind,
    /// Context error messages.
    pub contexts: Vec<Cow<'static, str>>,
    /// Error message from server or static if server couldn't respond.
    pub server_message: Option<Cow<'static, str>>,
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

#[derive(Debug)]
#[non_exhaustive]
pub enum Kind {
    /// Error returned by ffi calls
    Hapi(HapiResult),
    /// CString contains null byte
    NullByte(std::ffi::NulError),
    /// String is not a valid utf-8
    Utf8Error(std::string::FromUtf8Error),
    /// IO Error
    Io(std::io::Error),
    /// Misc error message from this crate.
    Internal(Cow<'static, str>),
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
            Kind::NullByte(_) => "String contains null byte!",
            Kind::Utf8Error(_) => "String is not UTF-8!",
            Kind::Internal(s) => s,
            Kind::Io(_) => "IO Error",
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

impl From<std::io::Error> for HapiError {
    fn from(value: std::io::Error) -> Self {
        HapiError {
            kind: Kind::Io(value),
            contexts: Vec::new(),
            server_message: None,
        }
    }
}

impl HapiError {
    pub(crate) fn new_hapi_with_server_message(
        result: HapiResult,
        server_message: Cow<'static, str>,
    ) -> Self {
        HapiError {
            kind: Kind::Hapi(result),
            contexts: vec![],
            server_message: Some(server_message),
        }
    }
    pub(crate) fn new(
        kind: Kind,
        mut static_message: Option<Cow<'static, str>>,
        server_message: Option<Cow<'static, str>>,
    ) -> HapiError {
        let mut contexts = vec![];
        if let Some(m) = static_message.take() {
            contexts.push(m);
        }
        HapiError {
            kind,
            contexts,
            server_message,
        }
    }
    pub(crate) fn internal<M: Into<Cow<'static, str>>>(message: M) -> Self {
        HapiError {
            kind: Kind::Internal(message.into()),
            server_message: None,
            contexts: vec![],
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
                    writeln!(f)?; // blank line
                }
                for (n, msg) in self.contexts.iter().enumerate() {
                    writeln!(f, "\t{}. {}", n, msg)?;
                }
                Ok(())
            }
            Kind::NullByte(e) => unsafe {
                let text = e.clone().into_vec();
                // SAFETY: We don't care about utf8 for error reporting I think
                let text = std::str::from_utf8_unchecked(&text);
                write!(f, "{e} in string \"{}\"", text)
            },
            Kind::Utf8Error(e) => unsafe {
                // SAFETY: We don't care about utf8 for error reporting I think
                let text = std::str::from_utf8_unchecked(e.as_bytes());
                write!(f, "{e} in string \"{}\"", text)
            },
            Kind::Io(e) => {
                write!(f, "{}", e)
            }
            Kind::Internal(e) => {
                write!(f, "[Internal Error]: {e}")
            }
        }
    }
}

impl std::fmt::Debug for HapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<std::ffi::NulError> for HapiError {
    fn from(e: std::ffi::NulError) -> Self {
        HapiError::new(Kind::NullByte(e), None, None)
    }
}

impl From<std::string::FromUtf8Error> for HapiError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        HapiError::new(Kind::Utf8Error(e), None, None)
    }
}

impl std::error::Error for HapiError {}

impl HapiResult {
    pub(crate) fn check_err<R: Default, F, M>(self, session: &Session, context: F) -> Result<R>
    where
        M: Into<Cow<'static, str>>,
        F: FnOnce() -> M,
    {
        match self {
            HapiResult::Success => Ok(R::default()),
            _err => {
                let server_message = session
                    .get_status_string(StatusType::CallResult, StatusVerbosity::All)
                    .unwrap_or_else(|_| String::from("Could not retrieve error message"));
                let mut err =
                    HapiError::new_hapi_with_server_message(self, Cow::Owned(server_message));
                err.contexts.push(context().into());
                Err(err)
            }
        }
    }

    pub(crate) fn error_message<I: Into<Cow<'static, str>>, R: Default>(
        self,
        message: I,
    ) -> Result<R> {
        match self {
            HapiResult::Success => Ok(R::default()),
            _err => {
                let mut err = HapiError::new_hapi_with_server_message(
                    self,
                    Cow::Borrowed("Server error message unavailable"),
                );
                err.contexts.push(message.into());
                Err(err)
            }
        }
    }
}
