mod ffi {
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

    pub fn HAPI_GeoInfoBuilder_Create() -> HAPI_GeoInfo {
        HAPI_GeoInfo::default()
    }
}

#[derive(Debug)]
struct GeoInfo(ffi::HAPI_GeoInfo);

impl GeoInfo {
    pub fn is_editable(&self) -> i32 {
        self.0.isEditable
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

impl Default for GeoInfoBuilder {
    fn default() -> Self {
        GeoInfoBuilder {
            inner: ffi::HAPI_GeoInfoBuilder_Create(),
        }
    }
}
