pub use hapi_sys::ffi;

#[macro_use]
mod macros;
#[macro_use]
mod structs;
mod asset;
mod cookoptions;
mod errors;
mod extentions;
mod node;
mod session;
mod status;
mod stringhandle;

pub use cookoptions::CookOptions;
pub use errors::{HapiError, Kind, Result};
pub(crate) use extentions::*;
pub use node::{Node, NodeInfo};
pub use session::{Initializer, Session};
pub use stringhandle::get_string;

fn foo() {

    use crate::structs::*;

    let h = HandleInfo(bla::HAPI_HandleInfo{});
}


#[cfg(test)]
mod test{
    use crate::NodeInfo;

    #[test]
    fn t() {

        use crate::structs::*;

        let h = HandleInfo();
    }

}


// impl ffi::HAPI_HandleInfo {
//     pub fn name(&self, session: *const ffi::HAPI_Session) -> Result<String> {
//         get_string(self.nameSH, session)
//     }
//
//     pub fn type_name(&self, session: *const ffi::HAPI_Session) -> Result<String> {
//         get_string(self.typeNameSH, session)
//     }
//
//     pub fn binding_count(&self) -> i32 {
//         self.bindingsCount
//     }
// }