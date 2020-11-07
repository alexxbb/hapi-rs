use paste::paste;
// pub struct HAPI_HandleInfo {
//     pub nameSH: HAPI_StringHandle,
//     pub typeNameSH: HAPI_StringHandle,
//     pub bindingsCount: ::std::os::raw::c_int,
// }

use crate::ffi;


pub mod bla {
    pub struct HAPI_HandleInfo{}
}

macro_rules! _impl {
    ($name:ident) => {
        paste! {
        pub struct $name(pub(crate) bla::[<HAPI_ $name>]);
        }
    };
}

_impl!(HandleInfo);
