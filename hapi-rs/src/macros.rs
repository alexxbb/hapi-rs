use crate::{node::NodeHandle, session::Session};

macro_rules! get {

    ($method:ident->$field:ident->bool) => {
        #[inline]
        pub fn $method(&self) -> bool {
            self.inner.$field == 1
        }
    };

    // wrap raw ids into handle i.e NodeHandle, ParmHandle etc
    ($method:ident->$field:ident->[handle: $hdl:ident]) => {
        #[inline]
        pub fn $method(&self) -> $hdl {
            $hdl(self.inner.$field)
        }
    };

    ($self_:ident, $method:ident->$block:block->$tp:ty) => {
        #[inline]
        pub fn $method(&$self_) -> $tp {
            $block
        }
    };

    ($method:ident->$field:ident->Result<String>) => {
        #[inline]
        pub fn $method(&self, session: &Session) -> Result<String> {
            session.get_string(self.inner.$field)
        }
    };

    ($method:ident->$field:ident->$tp:ty) => {
        #[inline]
        pub fn $method(&self) -> $tp {
            self.inner.$field
        }
    };

    ($method:ident->$field:ident->[$($tp:tt)*]) => {
        get!($method->$field->[$($tp)*]);
    };
}

macro_rules! setter {
    ($method:ident->$field:ident->bool) => {
        pub fn $method(mut self, val: bool) -> Self {
            self.0.$field = val as i8;
            self
        }
    };

    ($method:ident->$field:ident->$tp:ty) => {
        pub fn $method(mut self, val: $tp) -> Self {
            self.0.$field = val;
            self
        }
    };
}

macro_rules! wrap_ffi {
    (_get_ $method:ident->$field:ident->bool) => {
        get!($method->$field->bool);
    };

    (_get_ $method:ident->$field:ident->Result<String>) => {
        get!($method->$field->Result<String>);
    };

    (_set_ $method:ident->$field:ident->Result<String>) => {
        // Ignore string setter for builder
    };

    (_get_ $method:ident->$field:ident->$tp:ty) => {
        get!($method->$field->$tp);
    };

    (_set_ $method:ident->$field:ident->bool) => {
        pub fn $method(mut self, val: bool) -> Self {self.inner.$field = val as i8; self}
    };
    (_set_ $method:ident->$field:ident->$tp:ty) => {
        pub fn $method(mut self, val: $tp) -> Self {self.inner.$field = val; self}
    };

    // Entry point
    (
        @object: $object:ident
        @builder: $builder:ident
        @default: [$create_func:path=>$ffi_tp:ty]
        methods:
            $($method:ident->$field:ident->[$($tp:tt)*]);* $(;)?
    ) => {
        use crate::{ session::Session };
        pub struct $builder{inner: $ffi_tp }
        impl Default for $builder {
            fn default() -> Self {
                Self{inner: unsafe { $create_func() }}
            }
        }

        impl $builder {
            $(wrap_ffi!(_set_ $method->$field->$($tp)*);)*

            pub fn build(mut self) -> $object {
                $object{inner: self.inner}
            }
        }

        impl $object {
            $(wrap_ffi!(_get_ $method->$field->$($tp)*);)*

            pub fn ptr(&self) -> *const $ffi_tp {
                &self.inner as *const _
            }
        }

        impl Default for $object {
            fn default() -> Self {
                $builder::default().build()
            }
        }
    };
}

macro_rules! char_ptr {
    ($lit:expr) => {{
        use std::ffi::CStr;
        use std::os::raw::c_char;
        unsafe { CStr::from_ptr(concat!($lit, "\0").as_ptr() as *const c_char).as_ptr() }
    }};
}

macro_rules! check_session {
    ($session:expr) => {
        use crate::ffi::{HAPI_IsSessionValid, HapiResult};
        assert!(
            unsafe { matches!(HAPI_IsSessionValid($session), HapiResult::Success) },
            "Session is invalid!"
        );
    };
}
