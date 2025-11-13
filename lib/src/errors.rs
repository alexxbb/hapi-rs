use crate::session::Session;

pub use crate::ffi::raw::{HapiResult, StatusType, StatusVerbosity};
use std::borrow::Cow;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, HapiError>;

/// Error type returned by all APIs
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum HapiError {
    /// HAPI function call failed
    Hapi {
        result_code: HapiResultCode,
        server_message: Option<String>,
        contexts: Vec<String>,
    },

    /// CString conversion error - string contains null byte
    NullByte(#[from] std::ffi::NulError),

    /// UTF-8 conversion error
    Utf8(#[from] std::string::FromUtf8Error),

    /// IO error
    Io(#[from] std::io::Error),

    /// Internal library error
    Internal(String),
}

// Wrapper for HapiResult to provide Display for error messages
#[derive(Debug, Clone, Copy)]
pub struct HapiResultCode(pub HapiResult);

impl std::fmt::Display for HapiResultCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use HapiResult::*;
        let desc = match self.0 {
            Success => "SUCCESS",
            Failure => "FAILURE",
            AlreadyInitialized => "ALREADY_INITIALIZED",
            NotInitialized => "NOT_INITIALIZED",
            CantLoadfile => "CANT_LOADFILE",
            ParmSetFailed => "PARM_SET_FAILED",
            InvalidArgument => "INVALID_ARGUMENT",
            CantLoadGeo => "CANT_LOAD_GEO",
            CantGeneratePreset => "CANT_GENERATE_PRESET",
            CantLoadPreset => "CANT_LOAD_PRESET",
            AssetDefAlreadyLoaded => "ASSET_DEF_ALREADY_LOADED",
            NoLicenseFound => "NO_LICENSE_FOUND",
            DisallowedNcLicenseFound => "DISALLOWED_NC_LICENSE_FOUND",
            DisallowedNcAssetWithCLicense => "DISALLOWED_NC_ASSET_WITH_C_LICENSE",
            DisallowedNcAssetWithLcLicense => "DISALLOWED_NC_ASSET_WITH_LC_LICENSE",
            DisallowedLcAssetWithCLicense => "DISALLOWED_LC_ASSET_WITH_C_LICENSE",
            DisallowedHengineindieW3partyPlugin => "DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN",
            AssetInvalid => "ASSET_INVALID",
            NodeInvalid => "NODE_INVALID",
            UserInterrupted => "USER_INTERRUPTED",
            InvalidSession => "INVALID_SESSION",
            SharedMemoryBufferOverflow => "SHARED_MEMORY_BUFFER_OVERFLOW",
        };
        write!(f, "{}", desc)
    }
}

// This special case for TryFrom<T, Error = HapiError> where conversion can't fail.
// for example when "impl TryInto<AttributeName>" receives AttributeName.
impl From<std::convert::Infallible> for HapiError {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!()
    }
}

impl From<HapiResult> for HapiError {
    fn from(r: HapiResult) -> Self {
        HapiError::Hapi {
            result_code: HapiResultCode(r),
            server_message: None,
            contexts: Vec::new(),
        }
    }
}

impl From<&str> for HapiError {
    fn from(value: &str) -> Self {
        HapiError::Internal(value.to_string())
    }
}

impl HapiError {
    /// Create a new HAPI error with context and server message
    pub(crate) fn new(
        kind: Kind,
        static_message: Option<Cow<'static, str>>,
        server_message: Option<Cow<'static, str>>,
    ) -> HapiError {
        match kind {
            Kind::Hapi(result) => {
                let mut contexts = Vec::new();
                if let Some(msg) = static_message {
                    contexts.push(msg.into_owned());
                }
                HapiError::Hapi {
                    result_code: HapiResultCode(result),
                    server_message: server_message.map(|s| s.into_owned()),
                    contexts,
                }
            }
            Kind::NullByte(e) => HapiError::NullByte(e),
            Kind::Utf8Error(e) => HapiError::Utf8(e),
            Kind::Io(e) => HapiError::Io(e),
            Kind::Internal(s) => HapiError::Internal(s.into_owned()),
        }
    }

    /// Create an internal error
    pub(crate) fn internal<M: Into<String>>(message: M) -> Self {
        HapiError::Internal(message.into())
    }

    /// Get the kind for backwards compatibility
    pub fn kind(&self) -> Kind {
        match self {
            HapiError::Hapi { result_code, .. } => Kind::Hapi(result_code.0),
            HapiError::NullByte(e) => Kind::NullByte(e.clone()),
            HapiError::Utf8(e) => Kind::Utf8Error(e.clone()),
            HapiError::Io(e) => Kind::Io(std::io::Error::new(e.kind(), e.to_string())),
            HapiError::Internal(s) => Kind::Internal(Cow::Owned(s.clone())),
        }
    }

    /// Get contexts for HAPI errors
    pub fn contexts(&self) -> &[String] {
        match self {
            HapiError::Hapi { contexts, .. } => contexts,
            _ => &[],
        }
    }

    /// Get server message for HAPI errors
    pub fn server_message(&self) -> Option<&str> {
        match self {
            HapiError::Hapi { server_message, .. } => server_message.as_deref(),
            _ => None,
        }
    }
}

// Keep the Kind enum for backwards compatibility
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

pub(crate) trait ErrorContext<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Into<String>;

    #[allow(unused)]
    fn with_context<C, F>(self, func: F) -> Result<T>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T> ErrorContext<T> for Result<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: Into<String>,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(mut error) => {
                if let HapiError::Hapi { contexts, .. } = &mut error {
                    contexts.push(context.into());
                }
                Err(error)
            }
        }
    }

    fn with_context<C, F>(self, func: F) -> Result<T>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(mut error) => {
                if let HapiError::Hapi { contexts, .. } = &mut error {
                    contexts.push(func().into());
                }
                Err(error)
            }
        }
    }
}

// Custom Display to show contexts properly for HAPI errors
impl std::fmt::Display for HapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HapiError::Hapi {
                result_code,
                server_message,
                contexts,
            } => {
                write!(f, "[{}]", result_code)?;
                if let Some(msg) = server_message {
                    write!(f, ": [Engine Message]: {}", msg)?;
                }
                if !contexts.is_empty() {
                    writeln!(f)?;
                    for (n, msg) in contexts.iter().enumerate() {
                        writeln!(f, "\t{}. {}", n, msg)?;
                    }
                }
                Ok(())
            }
            HapiError::NullByte(e) => {
                let vec = e.clone().into_vec();
                let text = String::from_utf8_lossy(&vec);
                write!(f, "String contains null byte in \"{}\"", text)
            }
            HapiError::Utf8(e) => {
                let text = String::from_utf8_lossy(e.as_bytes());
                write!(f, "Invalid UTF-8 in string \"{}\"", text)
            }
            HapiError::Io(e) => write!(f, "IO error: {}", e),
            HapiError::Internal(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl HapiResult {
    pub(crate) fn check_err<F, M>(self, session: &Session, context: F) -> Result<()>
    where
        M: Into<String>,
        F: FnOnce() -> M,
    {
        match self {
            HapiResult::Success => Ok(()),
            _err => {
                let server_message = session
                    .get_status_string(StatusType::CallResult, StatusVerbosity::All)
                    .ok();
                
                Err(HapiError::Hapi {
                    result_code: HapiResultCode(self),
                    server_message: server_message.or_else(|| Some("Could not retrieve error message".to_string())),
                    contexts: vec![context().into()],
                })
            }
        }
    }

    pub(crate) fn error_message<I: Into<String>>(self, message: I) -> Result<()> {
        match self {
            HapiResult::Success => Ok(()),
            _err => {
                Err(HapiError::Hapi {
                    result_code: HapiResultCode(self),
                    server_message: Some("Server error message unavailable".to_string()),
                    contexts: vec![message.into()],
                })
            }
        }
    }
}
