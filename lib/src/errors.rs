use crate::session::Session;

pub use crate::ffi::raw::{HapiResult, StatusType, StatusVerbosity};
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

    /// This is used by [`ErrorContext::context`] / [`ErrorContext::with_context`]
    Context {
        contexts: Vec<String>,
        #[source]
        source: Box<HapiError>,
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
                let context = context.into();
                match &mut error {
                    HapiError::Hapi { contexts, .. } => {
                        contexts.push(context);
                        Err(error)
                    }
                    HapiError::Context { contexts, .. } => {
                        contexts.push(context);
                        Err(error)
                    }
                    _ => Err(HapiError::Context {
                        contexts: vec![context],
                        source: Box::new(error),
                    }),
                }
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
                let context = func().into();
                match &mut error {
                    HapiError::Hapi { contexts, .. } => {
                        contexts.push(context);
                        Err(error)
                    }
                    HapiError::Context { contexts, .. } => {
                        contexts.push(context);
                        Err(error)
                    }
                    _ => Err(HapiError::Context {
                        contexts: vec![context],
                        source: Box::new(error),
                    }),
                }
            }
        }
    }
}

// Custom Display to show contexts properly for HAPI errors
impl std::fmt::Display for HapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_base(err: &HapiError, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match err {
                HapiError::Hapi {
                    result_code,
                    server_message,
                    ..
                } => {
                    write!(f, "[{}]", result_code)?;
                    if let Some(msg) = server_message {
                        write!(f, ": [Engine Message]: {}", msg)?;
                    }
                    Ok(())
                }
                HapiError::Context { source, .. } => fmt_base(source, f),
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

        fn collect_contexts<'a>(err: &'a HapiError, out: &mut Vec<&'a str>) {
            match err {
                HapiError::Hapi { contexts, .. } => {
                    out.extend(contexts.iter().map(|s| s.as_str()));
                }
                HapiError::Context { contexts, source } => {
                    collect_contexts(source, out);
                    out.extend(contexts.iter().map(|s| s.as_str()));
                }
                _ => {}
            }
        }

        fmt_base(self, f)?;

        let mut contexts = Vec::new();
        collect_contexts(self, &mut contexts);
        if !contexts.is_empty() {
            writeln!(f)?;
            for (n, msg) in contexts.iter().enumerate() {
                writeln!(f, "\t{}. {}", n, msg)?;
            }
        }
        Ok(())
    }
}

impl HapiResult {
    /// Check HAPI_Result status and convert to HapiError if the status is not success.
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
                    server_message: server_message
                        .or_else(|| Some("Could not retrieve error message".to_string())),
                    contexts: vec![context().into()],
                })
            }
        }
    }

    /// Convert HAPI_Result to HapiError if the status is not success and add a message to the error.
    pub(crate) fn add_context<I: Into<String>>(self, message: I) -> Result<()> {
        match self {
            HapiResult::Success => Ok(()),
            _err => Err(HapiError::Hapi {
                result_code: HapiResultCode(self),
                server_message: None,
                contexts: vec![message.into()],
            }),
        }
    }

    pub(crate) fn with_context<F, M>(self, func: F) -> Result<()>
    where
        F: FnOnce() -> M,
        M: Into<String>,
    {
        self.add_context(func())
    }

    pub(crate) fn with_server_message<F, M>(self, func: F) -> Result<()>
    where
        F: FnOnce() -> M,
        M: Into<String>,
    {
        match self {
            HapiResult::Success => Ok(()),
            _err => Err(HapiError::Hapi {
                result_code: HapiResultCode(self),
                server_message: Some(func().into()),
                contexts: vec![],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn context_chain_is_rendered_for_internal_errors() {
        let err = (Err::<(), HapiError>(HapiError::Internal("root".to_string())))
            .context("first context")
            .context("second context")
            .unwrap_err();

        let s = err.to_string();
        assert!(s.starts_with("Internal error: root"));
        assert!(s.contains("\n\t0. first context\n\t1. second context\n"));

        // Verify the source chain is preserved via #[source]
        let source = err.source().expect("source");
        assert_eq!(source.to_string(), "Internal error: root");
    }

    #[test]
    fn hapi_errors_server_message_and_contexts() {
        let err = (Err::<(), HapiError>(HapiError::Hapi {
            result_code: HapiResultCode(HapiResult::Failure),
            server_message: Some("could not cook".to_string()),
            contexts: vec!["low-level".to_string()],
        }))
        .context("high-level")
        .unwrap_err();

        let s = err.to_string();
        assert_eq!(
            s,
            "[FAILURE]: [Engine Message]: could not cook\n\t0. low-level\n\t1. high-level\n"
        );
    }

    #[test]
    fn context_added_outside_hapi_error_is_rendered_after_inner_contexts() {
        // Create a HAPI error with one context, then add an outer wrapper context.
        let base = HapiError::Hapi {
            result_code: HapiResultCode(HapiResult::InvalidArgument),
            server_message: None,
            contexts: vec!["inner".to_string()],
        };
        let wrapped = (Err::<(), HapiError>(base)).context("outer").unwrap_err();

        let s = wrapped.to_string();
        // Base header comes from the underlying Hapi error
        assert!(s.starts_with("[INVALID_ARGUMENT]"));
        // Context order: inner first, then outer
        assert!(s.contains("\n\t0. inner\n\t1. outer\n"));
    }
}
