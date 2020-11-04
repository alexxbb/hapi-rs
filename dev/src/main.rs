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
        GeoInfoBuilder{inner: ffi::HAPI_GeoInfoBuilder_Create()}
    }
}

fn main() {
    let mut b = GeoInfoBuilder::default()
        .set_editable(true)
        .build();
    dbg!(b);
}
