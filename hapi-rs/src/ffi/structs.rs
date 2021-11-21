use super::raw::*;
use crate::{
    errors::Result,
    node::{HoudiniNode, NodeHandle},
    parameter::ParmHandle,
    session::Session,
};
use paste::paste;
use std::ffi::CString;

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
            $hdl(self.inner.$field, ())
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
        pub fn $method(&self) -> Result<String> {
            self.session.get_string(self.inner.$field)
        }
    };

    (with_session $method:ident->$field:ident->Result<String>) => {
        #[inline]
        pub fn $method(&self, session: &Session) -> Result<String> {
            session.get_string(self.inner.$field)
        }
    };

    ($method:ident->$field:ident->Result<CString>) => {
        #[inline]
        pub fn $method(&self) -> Result<CString> {
            crate::stringhandle::get_cstring(self.inner.$field, &self.session)
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

macro_rules! wrap {
    (_set_ $method:ident->$field:ident->bool) => {
        paste!{
            pub fn [<with_ $method>](mut self, val: bool) -> Self {self.inner.$field = val as i8; self}
        }
    };
    (_set_ $method:ident->$field:ident->$tp:ty) => {
        paste!{
            pub fn [<with_ $method>](mut self, val: $tp) -> Self {self.inner.$field = val; self}
        }
    };

    // impl [get|set]
    ([get] $object:ident $method:ident->$field:ident->$($tp:tt)*) => {
        get!($method->$field->$($tp)*);
    };

    ([get+session] $object:ident $method:ident->$field:ident->$($tp:tt)*) => {
        get!(with_session $method->$field->$($tp)*);
    };

    ([set] $object:ident $method:ident->$field:ident->$($tp:tt)*) => {
        $(wrap!{_set_ $method->$field->$tp})*
    };

    ([get|set] $object:ident $method:ident->$field:ident->$($tp:tt)*) => {
        get!($method->$field->$($tp)*);
        $(wrap!{_set_ $method->$field->$tp})*
    };

    (New $object:ident [$create_func:path=>$ffi_tp:ty]; $($rest:tt)*) => {
        impl $object {
            pub fn new(session: Session) -> Self {
                Self{inner: unsafe { $create_func() }, session}
            }
        }
        wrap!{_impl_methods_ $object $ffi_tp $($rest)*}
    };

    (Default $object:ident [$create_func:path=>$ffi_tp:ty]; $($rest:tt)*) => {
        impl Default for $object {
            fn default() -> Self {
                Self{inner: unsafe { $create_func() }}
            }
        }
        wrap!{_impl_methods_ $object $ffi_tp $($rest)*}
    };


    (_impl_methods_ $object:ident $ffi_tp:ty
        $([$($access:tt)*] $method:ident->$field:ident->[$($tp:tt)*]);* $(;)?
    ) => {
        impl $object {
            $(wrap!([$($access)*] $object $method->$field->$($tp)*);)*

            #[inline]
            pub fn ptr(&self) -> *const $ffi_tp {
                &self.inner as *const _
            }
        }
    };
}

#[derive(Debug)]
pub struct ParmChoiceInfo {
    pub(crate) inner: HAPI_ParmChoiceInfo,
    pub(crate) session: Session,
}

impl ParmChoiceInfo {
    get!(value->valueSH->Result<String>);
    get!(label->labelSH->Result<String>);
}

#[derive(Debug)]
pub struct ParmInfo {
    pub(crate) inner: HAPI_ParmInfo,
    pub(crate) session: Session,
}

impl ParmInfo {
    get!(id->id->[handle: ParmHandle]);
    get!(parent_id->parentId->[handle: ParmHandle]);
    get!(child_index->childIndex->i32);
    get!(parm_type->type_->ParmType);
    get!(script_type->scriptType->PrmScriptType);
    get!(permissions->permissions->Permissions);
    get!(tag_count->tagCount->i32);
    get!(size->size->i32);
    get!(choice_count->choiceCount->i32);
    get!(choice_list_type->choiceListType->ChoiceListType);
    get!(has_min->hasMin->bool);
    get!(has_max->hasMax->bool);
    get!(has_uimin->hasUIMin->bool);
    get!(has_uimax->hasUIMax->bool);
    get!(min->min->f32);
    get!(max->max->f32);
    get!(uimin->UIMin->f32);
    get!(uimax->UIMax->f32);
    get!(invisible->invisible->bool);
    get!(disabled->disabled->bool);
    get!(spare->spare->bool);
    get!(join_next->joinNext->bool);
    get!(label_none->labelNone->bool);
    get!(int_values_index->intValuesIndex->i32);
    get!(float_values_index->floatValuesIndex->i32);
    get!(string_values_index->stringValuesIndex->i32);
    get!(choice_index->choiceIndex->i32);
    get!(input_node_type->inputNodeType->NodeType);
    get!(input_node_flag->inputNodeFlag->NodeFlags);
    get!(is_child_of_multi_parm->isChildOfMultiParm->bool);
    get!(instance_num->instanceNum->i32);
    get!(instance_length->instanceLength->i32);
    get!(instance_count->instanceCount->i32);
    get!(instance_start_offset->instanceStartOffset->i32);
    get!(ramp_type->rampType->RampType);
    get!(type_info->typeInfoSH->Result<String>);
    get!(name->nameSH->Result<String>);
    get!(name_cstr->nameSH->Result<CString>);
    get!(label->labelSH->Result<String>);
    get!(template_name->templateNameSH->Result<String>);
    get!(help->helpSH->Result<String>);
    get!(visibility_condition->visibilityConditionSH->Result<String>);
    get!(disabled_condition->disabledConditionSH->Result<String>);
}

#[derive(Clone)]
pub struct NodeInfo {
    pub(crate) inner: HAPI_NodeInfo,
    pub(crate) session: Session,
}

impl NodeInfo {
    get!(name->nameSH->Result<String>);
    get!(internal_path->internalNodePathSH->Result<String>);
    get!(node_type->type_->NodeType);
    get!(is_valid->isValid->bool);
    get!(unique_node_id->uniqueHoudiniNodeId->i32);
    get!(total_cook_count->totalCookCount->i32);
    get!(child_node_count->childNodeCount->i32);
    get!(parm_count->parmCount->i32);
    get!(input_count->inputCount->i32);
    get!(output_count->outputCount->i32);
    get!(is_time_dependent->isTimeDependent->bool);
    get!(created_post_asset_load->createdPostAssetLoad->bool);
    get!(parm_int_value_count->parmIntValueCount->i32);
    get!(parm_float_value_count->parmFloatValueCount->i32);
    get!(parm_string_value_count->parmStringValueCount->i32);
    get!(parm_choice_count->parmChoiceCount->i32);
    get!(node_handle->id->[handle: NodeHandle]);
    get!(parent_id->parentId->[handle: NodeHandle]);
}

impl std::fmt::Debug for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeInfo")
            .field("name", &self.name().unwrap())
            .field("internal_path", &self.internal_path().unwrap())
            .field("type", &crate::node::node_type_name(self.node_type()))
            .field("is_valid", &self.is_valid())
            .field("time_dependent", &self.is_time_dependent())
            .field("total_cook_count", &self.total_cook_count())
            .field("parm_count", &self.parm_count())
            .field("child_count", &self.child_node_count())
            .field("input_count", &self.input_count())
            .field("output_count", &self.output_count())
            .finish()
    }
}

#[derive(Debug)]
pub struct CookOptions {
    pub(crate) inner: HAPI_CookOptions,
}

wrap!(
    Default CookOptions [HAPI_CookOptions_Create => HAPI_CookOptions];
    [set] split_geo_by_group->splitGeosByGroup->[bool];
    [set] split_geos_by_attribute->splitGeosByAttribute->[bool];
    [set] max_vertices_per_primitive->maxVerticesPerPrimitive->[i32];
    [set] refine_curve_to_linear->refineCurveToLinear->[bool];
    [set] curve_refine_lod->curveRefineLOD->[f32];
    [set] clear_errors_and_warnings->clearErrorsAndWarnings->[bool];
    [set] cook_templated_geos->cookTemplatedGeos->[bool];
    [set] split_points_by_vertex_attributes->splitPointsByVertexAttributes->[bool];
    [set] handle_box_part_types->handleBoxPartTypes->[bool];
    [set] handle_sphere_part_types->handleSpherePartTypes->[bool];
    [set] check_part_changes->checkPartChanges->[bool];
    [set] packed_prim_instancing_mode->packedPrimInstancingMode->[PackedPrimInstancingMode];
    [get+session] split_attr->splitAttrSH->[Result<String>];
    [set] extra_flags->extraFlags->[i32];
);

#[derive(Debug)]
pub struct AttributeInfo {
    pub(crate) inner: HAPI_AttributeInfo,
}

wrap!(
    Default AttributeInfo [HAPI_AttributeInfo_Create => HAPI_AttributeInfo];
    [get] exists->exists->[bool];
    [get] original_owner->originalOwner->[AttributeOwner];
    [get] total_array_elements->totalArrayElements->[i64];
    [get|set] owner->owner->[AttributeOwner];
    [get|set] storage->storage->[StorageType];
    [get|set] tuple_size->tupleSize->[i32];
    [get|set] type_info->typeInfo->[AttributeTypeInfo];
    [get|set] count->count->[i32];
);

#[derive(Debug)]
pub struct AssetInfo<'session> {
    pub(crate) inner: HAPI_AssetInfo,
    pub session: &'session Session,
}

impl<'s> AssetInfo<'s> {
    get!(node_id->nodeId->[handle: NodeHandle]);
    get!(object_node_id->objectNodeId->[handle: NodeHandle]);
    get!(has_ever_cooked->hasEverCooked->bool);
    get!(have_objects_changed->haveObjectsChanged->bool);
    get!(have_materials_changed->haveMaterialsChanged->bool);
    get!(object_count->objectCount->i32);
    get!(handle_count->handleCount->i32);
    get!(transform_input_count->transformInputCount->i32);
    get!(geo_input_count->geoInputCount->i32);
    get!(geo_output_count->geoOutputCount->i32);
    get!(name->nameSH->Result<String>);
    get!(label->labelSH->Result<String>);
    get!(file_path->filePathSH->Result<String>);
    get!(version->versionSH->Result<String>);
    get!(full_op_name->fullOpNameSH->Result<String>);
    get!(help_text->helpTextSH->Result<String>);
    get!(help_url->helpURLSH->Result<String>);
}

#[derive(Debug)]
pub struct ObjectInfo<'session> {
    pub(crate) inner: HAPI_ObjectInfo,
    pub session: &'session Session,
}

impl<'s> ObjectInfo<'s> {
    get!(name->nameSH->Result<String>);
    get!(object_instance_path->objectInstancePathSH->Result<String>);
    get!(has_transform_changed->hasTransformChanged->bool);
    get!(have_geos_changed->haveGeosChanged->bool);
    get!(is_visible->isVisible->bool);
    get!(is_instancer->isInstancer->bool);
    get!(is_instanced->isInstanced->bool);
    get!(geo_count->geoCount->bool);
    get!(node_id->nodeId->[handle: NodeHandle]);
    get!(object_to_instance_id->objectToInstanceId->[handle: NodeHandle]);
    pub fn to_node(&self) -> Result<HoudiniNode> {
        self.node_id().to_node(self.session)
    }
}

#[derive(Debug)]
pub struct GeoInfo<'session> {
    pub(crate) inner: HAPI_GeoInfo,
    pub session: &'session Session,
}

impl<'s> GeoInfo<'s> {
    get!(geo_type->type_->GeoType);
    get!(name->nameSH->Result<String>);
    get!(node_id->nodeId->[handle: NodeHandle]);
    get!(is_editable->isEditable->bool);
    get!(is_templated->isTemplated->bool);
    get!(is_display_geo->isDisplayGeo->bool);
    get!(has_geo_changed->hasGeoChanged->bool);
    get!(has_material_changed->hasMaterialChanged->bool);
    get!(point_group_count->pointGroupCount->i32);
    get!(primitive_group_count->primitiveGroupCount->i32);
    get!(part_count->partCount->i32);
}

#[derive(Debug)]
pub struct PartInfo {
    pub(crate) inner: HAPI_PartInfo,
}
wrap!(
    Default PartInfo [HAPI_PartInfo_Create => HAPI_PartInfo];
    [get] part_id->id->[i32];
    [get] attribute_counts->attributeCounts->[[i32; 4]];
    [get] has_changed->hasChanged->[bool];
    [get] is_instanced->isInstanced->[bool];
    [get+session] name->nameSH->[Result<String>];
    [get|set] part_type->type_->[PartType];
    [get|set] face_count->faceCount->[i32];
    [get|set] point_count->pointCount->[i32];
    [get|set] vertex_count->vertexCount->[i32];
    [get|set] instance_count->instanceCount->[i32];
    [get|set] instance_part_count->instancedPartCount->[i32];
);

#[derive(Debug, Clone)]
pub struct TimelineOptions {
    pub(crate) inner: HAPI_TimelineOptions,
}

wrap!(
    Default TimelineOptions [HAPI_TimelineOptions_Create => HAPI_TimelineOptions];
    [get|set] fps->fps->[f32];
    [get|set] start_time->startTime->[f32];
    [get|set] end_time->endTime->[f32];
);

#[derive(Debug, Clone)]
pub struct CurveInfo {
    pub(crate) inner: HAPI_CurveInfo,
}

wrap!(
    Default CurveInfo [HAPI_CurveInfo_Create => HAPI_CurveInfo];
    [get|set] curve_type->curveType->[CurveType];
    [get|set] curve_count->curveCount->[i32];
    [get|set] vertex_count->vertexCount->[i32];
    [get|set] knot_count->knotCount->[i32];
    [get|set] periodic->isPeriodic->[bool];
    [get|set] rational->isRational->[bool];
    [get|set] has_knots->hasKnots->[bool];
    [get|set] order->order->[i32];
);

#[derive(Debug, Clone)]
pub struct Viewport {
    pub(crate) inner: HAPI_Viewport,
}

wrap!(
    Default Viewport [HAPI_Viewport_Create => HAPI_Viewport];
    [get|set] position->position->[[f32; 3]];
    [get|set] rotation->rotationQuaternion->[[f32; 4]];
    [get|set] offset->offset->[f32];
);

#[derive(Debug, Clone)]
pub struct Transform {
    pub(crate) inner: HAPI_Transform,
}

wrap!(
    Default Transform [HAPI_Transform_Create => HAPI_Transform];
    [get|set] position->position->[[f32;3]];
    [get|set] rotation->rotationQuaternion->[[f32;4]];
    [get|set] scale->scale->[[f32;3]];
    [get|set] shear->shear->[[f32;3]];
    [get|set] rst_order->rstOrder->[RSTOrder];
);

#[derive(Debug, Clone)]
pub struct TransformEuler {
    pub(crate) inner: HAPI_TransformEuler,
}

wrap!(
    Default TransformEuler [HAPI_TransformEuler_Create => HAPI_TransformEuler];
    [get|set] position->position->[[f32;3]];
    [get|set] rotation->rotationEuler->[[f32;3]];
    [get|set] scale->scale->[[f32;3]];
    [get|set] shear->shear->[[f32;3]];
    [get|set] roation_order->rotationOrder->[XYZOrder];
    [get|set] rst_order->rstOrder->[RSTOrder];
);
