use super::raw::*;
use crate::{
    errors::Result,
    session::Session,
    node::{HoudiniNode, NodeHandle},
    parameter::ParmHandle,
};
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
        pub fn $method(&self) -> Result<String> {
            self.session.get_string(self.inner.$field)
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

macro_rules! builder {
    (_get_ $method:ident->$field:ident->bool) => {
        get!($method->$field->bool);
    };

    (_get_ $method:ident->$field:ident->Result<String>) => {
        pub fn $method(&self, session: &Session) -> Result<String> {
            session.get_string(self.inner.$field)
        }
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
        pub struct $builder{inner: $ffi_tp }
        impl Default for $builder {
            fn default() -> Self {
                Self{inner: unsafe { $create_func() }}
            }
        }

        impl $builder {
            $(builder!(_set_ $method->$field->$($tp)*);)*

            pub fn build(mut self) -> $object {
                $object{inner: self.inner}
            }
        }

        impl $object {
            $(builder!(_get_ $method->$field->$($tp)*);)*

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


#[derive(Debug)]
pub struct ParmChoiceInfo<'s> {
    pub(crate) inner: HAPI_ParmChoiceInfo,
    pub(crate) session: &'s Session
}

impl<'s> ParmChoiceInfo<'_> {
    get!(value->valueSH->Result<String>);
    get!(label->labelSH->Result<String>);
}

#[derive(Debug)]
pub struct ParmInfo<'session> {
    pub(crate) inner: HAPI_ParmInfo,
    pub(crate) session: &'session Session,
    pub(crate) name: Option<CString>,
}


impl<'session> ParmInfo<'session> {
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

#[derive(Debug)]
pub struct CookOptions {
    inner: HAPI_CookOptions,
}

builder!(
    @object: CookOptions
    @builder: CookOptionsBuilder
    @default: [HAPI_CookOptions_Create => HAPI_CookOptions]
    methods:
        split_geo_by_group->splitGeosByGroup->[bool];
        split_geos_by_attribute->splitGeosByAttribute->[bool];
        max_vertices_per_primitive->maxVerticesPerPrimitive->[i32];
        refine_curve_to_linear->refineCurveToLinear->[bool];
        curve_refine_lod->curveRefineLOD->[f32];
        clear_errors_and_warnings->clearErrorsAndWarnings->[bool];
        cook_templated_geos->cookTemplatedGeos->[bool];
        split_points_by_vertex_attributes->splitPointsByVertexAttributes->[bool];
        handle_box_part_types->handleBoxPartTypes->[bool];
        handle_sphere_part_types->handleSpherePartTypes->[bool];
        check_part_changes->checkPartChanges->[bool];
        packed_prim_instancing_mode->packedPrimInstancingMode->[PackedPrimInstancingMode];
        split_attr->splitAttrSH->[Result<String>];
        extra_flags->extraFlags->[i32]);

#[derive(Debug)]
pub struct AttributeInfo {
    pub(crate) inner: HAPI_AttributeInfo,
}


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
}
