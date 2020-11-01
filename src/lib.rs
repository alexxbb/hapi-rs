#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::all)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[macro_use]
mod macros;
mod asset;
mod cookoptions;
mod errors;
mod extentions;
mod node;
mod session;
mod status;
mod stringhandle;

pub use cookoptions::CookOptions;
pub use errors::{HAPI_Error, Kind, Result};
pub(crate) use extentions::*;
pub use node::{Node, NodeInfo};
pub use session::{Initializer, Session};
pub use stringhandle::get_string;


impl ffi::HAPI_HandleInfo {
    pub fn name(&self, session: *const ffi::HAPI_Session) -> Result<String> {
        get_string(self.nameSH, session)
    }

    pub fn type_name(&self, session: *const ffi::HAPI_Session) -> Result<String> {
        get_string(self.typeNameSH, session)
    }

    pub fn binding_count(&self) -> i32 {
        self.bindingsCount
    }
}