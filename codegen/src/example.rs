mod ffi {
    pub type HAPI_StringHandle = i32;
    pub struct Session{}
    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct HAPI_GeoInfo {
        pub type_: HAPI_GeoType,
        pub nameSH: HAPI_StringHandle,
        pub nodeId: HAPI_NodeId,
        pub isEditable: HAPI_Bool,
        pub isTemplated: HAPI_Bool,
        pub isDisplayGeo: HAPI_Bool,
        pub hasGeoChanged: HAPI_Bool,
        pub hasMaterialChanged: HAPI_Bool,
        pub pointGroupCount: ::std::os::raw::c_int,
        pub primitiveGroupCount: ::std::os::raw::c_int,
        pub partCount: ::std::os::raw::c_int,
    }

    pub enum HAPI_PartType {
        HAPI_PARTTYPE_INVALID = -1,
        HAPI_PARTTYPE_MESH = 0,
        HAPI_PARTTYPE_CURVE = 1,

    }
    pub fn HAPI_GeoInfoBuilder_Create() -> HAPI_GeoInfo {
        HAPI_GeoInfo::default()
    }

}


#[derive(Debug)]
pub struct GeoInfo {
    inner: ffi::HAPI_GeoInfo,
    session: ffi::Session
}

impl GeoInfo {
    pub fn is_editable(&self) -> i32 {
        self.inner.isEditable
    }
    pub fn name(&self) -> String {
        self.eval_string(self.inner.nameSH, self.session)
    }
}

pub trait HAPI_StringEval {
    fn eval_string(hdl: ffi::HAPI_StringHandle, session: *const ffi::Session);
}

impl HAPI_StringEval for GeoInfo {
    fn eval_string(hdl: ffi::HAPI_StringHandle, session: *const ffi::Session) {
        unimplemented!()
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
