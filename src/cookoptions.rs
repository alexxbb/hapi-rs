use crate::ffi;

pub enum PackedPrimInstancingMode {
    INVALID,
    DISABLED,
    HIERARCHY,
    FLAT,
    MAX,
}

pub struct CookOptions {
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

impl Default for CookOptions {
    fn default() -> CookOptions {
        CookOptions { inner: unsafe { ffi::HAPI_CookOptions_Create() } }
    }
}

impl CookOptions {
    
    #[inline]
    pub fn ptr(&self) -> *const ffi::HAPI_CookOptions {
        &self.inner as *const ffi::HAPI_CookOptions
    }

    // pub fn split_geos_by_group(mut self, val: bool) -> Self {
    //     self.inner.splitGeosByGroup = val as i8;
    //     self
    // }
    // pub fn split_geos_by_attribute(mut self, val: bool) -> Self {
    //     self.inner.splitGeosByAttribute = val as i8;
    //     self
    // }
    // pub fn max_vertices_per_primitive(mut self, val: i32) -> Self {
    //     self.inner.maxVerticesPerPrimitive = val;
    //     self
    // }
    // pub fn refine_curve_to_linear(mut self, val: bool) -> Self {
    //     self.inner.refineCurveToLinear = val as i8;
    //     self
    // }
    // pub fn packed_prim_instancing_mode(mut self, mode: PackedPrimInstancingMode) -> Self {
    //     self.inner.packedPrimInstancingMode = match mode {
    //         ffi::HAPI_PackedPrimInstancingMode::HAPI_PACKEDPRIM_INSTANCING_MODE_DISABLED
    //     };
    //     self
    // }
    // TODO the rest
    // pub curve_refine_lod: f32,
    // pub clear_errors_and_warnings: bool,
    // pub cook_templated_geos: bool,
    // pub split_points_by_vertex_attributes: bool,
    // pub handle_box_part_types: bool,
    // pub handle_sphere_part_types: bool,
    // pub check_part_changes: bool,
    // pub extra_flags: i32,

    // pub fn build(self) -> ffi::HAPI_CookOptions {
    //     self.inner
    // }
}