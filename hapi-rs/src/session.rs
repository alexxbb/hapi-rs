mod session;
mod cookoptions;
mod extra;

pub use session::{Session, SessionOptions, CookResult, StatusVerbosity};
pub use cookoptions::CookOptions;

#[cfg(test)]
mod session_tests {
    use crate::*;

    #[test]
    fn new_in_process() {
        let mut s = session::Session::new_in_process().expect("New session failed");
        s.initialize(session::SessionOptions::default()).expect("Could not initialize");
    }

    #[test]
    fn new_start_server() {
        let mut s = session::Session::start_named_pipe_server("/tmp/hapi-rs-test");
        s.initialize(session::SessionOptions::default()).expect("Could not initialize");
    }

}