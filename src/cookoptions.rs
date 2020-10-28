use crate::ffi;

pub enum PackedPrimInstancingMode {
    INVALID,
    DISABLED,
    HIERARCHY,
    FLAT,
    MAX
}

impl PackedPrimInstancingMode {
    fn index(&self) -> i32 {
        match *self {
            PackedPrimInstancingMode::INVALID => {-1}
            PackedPrimInstancingMode::DISABLED => {0}
            PackedPrimInstancingMode::HIERARCHY => {1}
            PackedPrimInstancingMode::FLAT => {2}
            PackedPrimInstancingMode::MAX => {3}
        }
    }
}

pub struct CookOptionsBuilder {
    inner: ffi::HAPI_CookOptions,
    // pub split_geos_by_group: bool,
    // pub split_geos_by_attribute: bool,
    // pub max_vertices_per_primitive: i32,
    // pub refine_curve_to_linear: bool,
    // pub curve_refine_lod: f32,
    // pub clear_errors_and_warnings: bool,
    // pub cook_templated_geos: bool,
    // pub split_points_by_vertex_attributes: bool,
    // pub packed_prim_instancing_mode: PackedPrimInstancingMode,
    // pub handle_box_part_types: bool,
    // pub handle_sphere_part_types: bool,
    // pub check_part_changes: bool,
    // pub extra_flags: i32,
}

impl CookOptionsBuilder {
    pub fn new() -> CookOptionsBuilder {
        CookOptionsBuilder {inner: unsafe {ffi::HAPI_CookOptions_Create()}  }
    }
    pub fn split_geos_by_group(mut self, val: bool) -> Self {
        self.inner.splitGeosByGroup = val as i8;
        self
    }
    pub fn split_geos_by_attribute(mut self, val: bool) -> Self {
        self.inner.splitGeosByAttribute = val as i8;
        self
    }
    pub fn max_vertices_per_primitive(mut self, val: i32) -> Self {
        self.inner.maxVerticesPerPrimitive = val;
        self
    }
    pub fn refine_curve_to_linear(mut self, val: bool) -> Self {
        self.inner.refineCurveToLinear = val as i8;
        self
    }
    pub fn packed_prim_instancing_mode(mut self, val: PackedPrimInstancingMode) -> Self {
        self.inner.packedPrimInstancingMode = val.index();
        self
    }
    // TODO the rest
    // pub curve_refine_lod: f32,
    // pub clear_errors_and_warnings: bool,
    // pub cook_templated_geos: bool,
    // pub split_points_by_vertex_attributes: bool,
    // pub handle_box_part_types: bool,
    // pub handle_sphere_part_types: bool,
    // pub check_part_changes: bool,
    // pub extra_flags: i32,

    pub fn build(self) -> ffi::HAPI_CookOptions {
        self.inner
    }
}