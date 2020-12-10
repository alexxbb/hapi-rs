use crate::ffi::{
    HAPI_CookOptions, HAPI_CookOptions_Create, PackedPrimInstancingMode, StorageType,
};

/*

   pub splitGeosByGroup: HAPI_Bool,
   pub splitGeosByAttribute: HAPI_Bool,
   pub splitAttrSH: HAPI_StringHandle,
   pub maxVerticesPerPrimitive: ::std::os::raw::c_int,
   pub refineCurveToLinear: HAPI_Bool,
   pub curveRefineLOD: f32,
   pub clearErrorsAndWarnings: HAPI_Bool,
   pub cookTemplatedGeos: HAPI_Bool,

   pub splitPointsByVertexAttributes: HAPI_Bool,

   pub packedPrimInstancingMode: PackedPrimInstancingMode,

   pub handleBoxPartTypes: HAPI_Bool,
   pub handleSpherePartTypes: HAPI_Bool,
   pub checkPartChanges: HAPI_Bool,
   pub extraFlags: ::std::os::raw::c_int,
*/

pub struct CookOptions {
    inner: HAPI_CookOptions,
}

wrap_ffi!(
    @object: CookOptions
    @builder: CookOptionsBuilder
    @ffi: [HAPI_CookOptions_Create => HAPI_CookOptions]
    methods:
        split_geo_by_group->splitGeosByGroup->bool;
        split_geos_by_attribute->splitGeosByAttribute->bool;
        max_vertices_per_primitive->maxVerticesPerPrimitive->i32;
        refine_curve_to_linear->refineCurveToLinear->bool;
        curve_refine_lod->curveRefineLOD->f32;
        clear_errors_and_warnings->clearErrorsAndWarnings->bool;
        cook_templated_geos->cookTemplatedGeos->bool;
        split_points_by_vertex_attributes->splitPointsByVertexAttributes->bool;
        handle_box_part_types->handleBoxPartTypes->bool;
        handle_sphere_part_types->handleSpherePartTypes->bool;
        check_part_changes->checkPartChanges->bool;
        packed_prim_instancing_mode->packedPrimInstancingMode->PackedPrimInstancingMode;
        extra_flags->extraFlags->i32);
