mod ffi {
    #[derive(Debug, Default)]
    pub struct HAPI_GeoInfo {
        pub type_: i32,
        pub nameSH: i32,
        pub nodeId: i32,
        pub isEditable: i32,
        pub isTemplated: i32,
        pub isDisplayGeo: i32,
        pub hasGeoChanged: i32,
        pub hasMaterialChanged: i32,
        pub pointGroupCount: i32,
        pub primitiveGroupCount: i32,
        pub partCount: i32,
    }

    pub fn HAPI_GeoInfoBuilder_Create() -> HAPI_GeoInfo {
        HAPI_GeoInfo::default()
    }
}
pub enum HAPI_License {
    HAPI_LICENSE_NONE = 0,
    HAPI_LICENSE_HOUDINI_ENGINE = 1,
}

pub enum Licence {
    NONE = 0,
    HOUDINI_ENGINE = 1,
}

impl From<HAPI_License> for Licence {
    fn from(e: HAPI_License) -> Self {
        match e {
            HAPI_License::HAPI_LICENSE_NONE => Licence::NONE,
            HAPI_License::HAPI_LICENSE_HOUDINI_ENGINE => Licence::HOUDINI_ENGINE,
        }
    }
}

impl From<Licence> for HAPI_License {
    fn from(e: Licence) -> Self {
        match e {
            Licence::HOUDINI_ENGINE => HAPI_License::HAPI_LICENSE_HOUDINI_ENGINE,
            Licence::NONE => HAPI_License::HAPI_LICENSE_NONE,
        }
    }
}

// impl From<HAPI_License> for Licence {
//     fn from(e: HAPI_License) -> Self {
//         match e {
//             HAPI_License::HAPI_LICENSE_NONE => Licence::NONE,
//             HAPI_License::HAPI_LICENSE_HOUDINI_ENGINE => Licence::HOUDINI_ENGINE,
//         }
//     }
//
// }

pub trait StringEval {
    fn eval_string(&self, hdl: i32, session: &str) -> String;
}

#[derive(Debug)]
struct GeoInfo(ffi::HAPI_GeoInfo);

impl GeoInfo {
    pub fn is_editable(&self) -> i32 {
        self.0.isEditable
    }
    pub fn name(&self) -> String {
        self.eval_string(10, "Hello")
    }
}

impl StringEval for GeoInfo {
    fn eval_string(&self, hdl: i32, session: &str) -> String {
        todo!()
    }
}

struct GeoInfoBuilder {
    inner: ffi::HAPI_GeoInfo,
}

impl GeoInfoBuilder {
    fn build(mut self) -> GeoInfo {
        GeoInfo(self.inner)
    }

    fn set_editable(mut self, val: bool) -> Self {
        self.inner.isEditable = val as i32;
        self
    }
}

impl Default for GeoInfoBuilder {
    fn default() -> Self {
        GeoInfoBuilder {
            inner: ffi::HAPI_GeoInfoBuilder_Create(),
        }
    }
}

fn main() {
    let mut b = GeoInfoBuilder::default().set_editable(true).build();
    // dbg!(b);
    let s = Licence::HOUDINI_ENGINE;
    let s: HAPI_License = s.into();
}
