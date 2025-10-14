use super::raw::*;
use crate::{
    errors::Result,
    node::{HoudiniNode, NodeHandle},
    parameter::ParmHandle,
    pdg::WorkItemId,
    session::Session,
    stringhandle::StringHandle,
};
use debug_ignore::DebugIgnore;
use pastey::paste;
use std::ffi::{CStr, CString};

macro_rules! get {

    ($method:ident->$field:ident->bool) => {
        #[inline]
        pub fn $method(&self) -> bool {
            self.0.$field == 1
        }
    };

    // wrap raw ids into handle i.e NodeHandle, ParmHandle etc
    ($method:ident->$field:ident->[handle: $hdl:ident]) => {
        #[inline]
        pub fn $method(&self) -> $hdl {
            $hdl(self.0.$field)
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
            use crate::stringhandle::StringHandle;
            crate::stringhandle::get_string(StringHandle(self.0.$field), &self.1)
        }
    };

    (with_session $method:ident->$field:ident->Result<String>) => {
        #[inline]
        pub fn $method(&self, session: &Session) -> Result<String> {
            use crate::stringhandle::StringHandle;
            crate::stringhandle::get_string(StringHandle(self.0.$field), session)
        }
    };

    ($method:ident->$field:ident->Result<CString>) => {
        #[inline]
        pub fn $method(&self) -> Result<CString> {
            use crate::stringhandle::StringHandle;
            crate::stringhandle::get_cstring(StringHandle(self.0.$field), &self.1)
        }
    };

    ($method:ident->$field:ident->$tp:ty) => {
        #[inline]
        pub fn $method(&self) -> $tp {
            self.0.$field
        }
    };

    ($method:ident->$field:ident->[$($tp:tt)*]) => {
        get!($method->$field->[$($tp)*]);
    };
}
// Impl Default trait for struct
// Default StructName [HapiFunction => HapiType];
// Example: Default CurveInfo [HAPI_CurveInfo_Create => HAPI_CurveInfo];
//
// Generate getters, setters and with ("builder") methods
// [get|set|with] struct_field->ffiStructField->[ValueType];
//  get:
//      fn get_struct_field(&self) -> ValueType { self.ffiStructField }
//  set:
//      fn set_struct_field(&self, val: ValueType)  { self.ffiStructField = val; }
//  with:
//      fn with_struct_field(self, val: ValueType) -> Self  { self.ffiStructField = val; self }
//
// Special case for string handles:
// [get+session] name->name->[ValueType]
// fn get_name(&self, session: &Session) -> Result<String> { session.get_string(self.0.name) }
//

macro_rules! wrap {
    (_with_ $method:ident->$field:ident->bool) => {
        paste!{
            pub fn [<with_ $method>](mut self, val: bool) -> Self {self.0.$field = val as i8; self}
        }
    };
    (_with_ $method:ident->$field:ident->$tp:ty) => {
        paste!{
            pub fn [<with_ $method>](mut self, val: $tp) -> Self {self.0.$field = val; self}
        }
    };
    (_set_ $method:ident->$field:ident->bool) => {
        paste!{
            pub fn [<set_ $method>](&mut self, val: bool)  {self.0.$field = val as i8}
        }
    };
    (_set_ $method:ident->$field:ident->$tp:ty) => {
        paste!{
            pub fn [<set_ $method>](&mut self, val: $tp)  {self.0.$field = val}
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

    ([with] $object:ident $method:ident->$field:ident->$($tp:tt)*) => {
        $(wrap!{_with_ $method->$field->$tp})*
    };

    ([get|set] $object:ident $method:ident->$field:ident->$($tp:tt)*) => {
        get!($method->$field->$($tp)*);
        $(wrap!{_set_ $method->$field->$tp})*
    };
    ([get|set|with] $object:ident $method:ident->$field:ident->$($tp:tt)*) => {
        get!($method->$field->$($tp)*);
        $(wrap!{_set_ $method->$field->$tp})*
        $(wrap!{_with_ $method->$field->$tp})*
    };

    (Default $object:ident [$create_func:path=>$ffi_tp:ty]; $($rest:tt)*) => {
        impl Default for $object {
            fn default() -> Self {
                #[allow(unused_unsafe)]
                Self(unsafe { $create_func() })
            }
        }
        wrap!{_impl_methods_ $object $ffi_tp $($rest)*}
    };

    (impl $object:ident=>$ffi_tp:ty; $($rest:tt)*) => {
        wrap!{_impl_methods_ $object $ffi_tp $($rest)*}
    };


    (_impl_methods_ $object:ident $ffi_tp:ty
        $([$($access:tt)*] $method:ident->$field:ident->[$($tp:tt)*]);* $(;)?
    ) => {
        impl $object {
            $(wrap!([$($access)*] $object $method->$field->$($tp)*);)*

            #[inline]
            pub fn ptr(&self) -> *const $ffi_tp {
                &self.0 as *const _
            }
        }
    };
}

/// Configurations for sessions.
/// Note: For async attribute access, make sure to set connection_count to at least 1.
#[derive(Clone, Debug)]
pub struct SessionInfo(pub(crate) HAPI_SessionInfo);

wrap! {
    Default SessionInfo [HAPI_SessionInfo_Create => HAPI_SessionInfo];
    [get|set|with] connection_count->connectionCount->[i32];
    [get|set|with] port_type->portType->[TcpPortType];
    [get] min_port->minPort->[i32];
    [get] max_port->maxPort->[i32];
    [get] ports->ports->[[i32;128usize]];
    [get|set|with] shared_memory_buffer_type->sharedMemoryBufferType->[ThriftSharedMemoryBufferType];
    [get|set|with] shared_memory_buffer_size->sharedMemoryBufferSize->[i64];
}

/// Options to configure a Thrift server being started from HARC.
#[derive(Clone)]
pub struct ThriftServerOptions(pub(crate) HAPI_ThriftServerOptions);

wrap! {
    Default ThriftServerOptions [HAPI_ThriftServerOptions_Create => HAPI_ThriftServerOptions];
    [get|set|with] auto_close->autoClose->[bool];
    [get|set|with] timeout_ms->timeoutMs->[f32];
    [get|set|with] verbosity->verbosity->[StatusVerbosity];
    [get|set|with] shared_memory_buffer_type->sharedMemoryBufferType->[ThriftSharedMemoryBufferType];
    [get|set|with] shared_memory_buffer_size->sharedMemoryBufferSize->[i64];
}

#[derive(Clone)]
pub struct CompositorOptions(pub(crate) HAPI_CompositorOptions);

wrap! {
    Default CompositorOptions [HAPI_CompositorOptions_Create => HAPI_CompositorOptions];
    [get|set] max_resolution_x->maximumResolutionX->[i32];
    [get|set] max_resolution_y->maximumResolutionY->[i32];
}

/// Menu parameter label and value
#[derive(Clone)]
pub struct ParmChoiceInfo(pub(crate) HAPI_ParmChoiceInfo, pub(crate) Session);

impl std::fmt::Debug for ParmChoiceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::borrow::Cow;

        let get_str = |h: i32| -> Cow<str> {
            match crate::stringhandle::get_string_bytes(StringHandle(h), &self.1) {
                // SAFETY: Don't care about utf in Debug
                Ok(bytes) => unsafe { Cow::Owned(String::from_utf8_unchecked(bytes)) },
                Err(_) => Cow::Borrowed("!!! Could not retrieve string"),
            }
        };

        f.debug_struct("ParmChoiceInfo")
            .field("label", &get_str(self.0.labelSH))
            .field("value", &get_str(self.0.valueSH))
            .finish()
    }
}

impl ParmChoiceInfo {
    get!(value->valueSH->Result<String>);
    get!(label->labelSH->Result<String>);
}

/// [Documentation](https://www.sidefx.com/docs/hengine/struct_h_a_p_i___parm_info.html)
#[derive(Debug)]
pub struct ParmInfo(
    pub(crate) HAPI_ParmInfo,
    pub(crate) DebugIgnore<Session>,
    pub(crate) Option<CString>,
);

impl ParmInfo {
    pub(crate) fn new(inner: HAPI_ParmInfo, session: Session, name: Option<CString>) -> Self {
        Self(inner, DebugIgnore(session), name)
    }
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

// #[derive(Clone)]
/// [Documentation](https://www.sidefx.com/docs/hengine/struct_h_a_p_i___node_info.html)
pub struct NodeInfo(pub(crate) HAPI_NodeInfo, pub(crate) DebugIgnore<Session>);

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

    pub(crate) fn new(session: &Session, node: NodeHandle) -> Result<Self> {
        let session = session.clone();
        let inner = crate::ffi::get_node_info(node, &session)?;
        Ok(Self(inner, DebugIgnore(session)))
    }
}

impl std::fmt::Debug for NodeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = "Error in Debug impl";
        f.debug_struct("NodeInfo")
            .field("name", &self.name().as_deref().unwrap_or(err))
            .field(
                "internal_path",
                &self.internal_path().as_deref().unwrap_or(err),
            )
            .field("type", &self.node_type())
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

/// [Documentation](https://www.sidefx.com/docs/hengine/struct_h_a_p_i___cook_options.html)
#[derive(Debug, Clone)]
pub struct CookOptions(pub(crate) HAPI_CookOptions);

wrap!(
    Default CookOptions [HAPI_CookOptions_Create => HAPI_CookOptions];
    [get|set|with] split_geo_by_group->splitGeosByGroup->[bool];
    [get|set|with] split_geos_by_attribute->splitGeosByAttribute->[bool];
    [get|set|with] max_vertices_per_primitive->maxVerticesPerPrimitive->[i32];
    [get|set|with] refine_curve_to_linear->refineCurveToLinear->[bool];
    [get|set|with] curve_refine_lod->curveRefineLOD->[f32];
    [get|set|with] clear_errors_and_warnings->clearErrorsAndWarnings->[bool];
    [get|set|with] cook_templated_geos->cookTemplatedGeos->[bool];
    [get|set|with] split_points_by_vertex_attributes->splitPointsByVertexAttributes->[bool];
    [get|set|with] handle_box_part_types->handleBoxPartTypes->[bool];
    [get|set|with] handle_sphere_part_types->handleSpherePartTypes->[bool];
    [get|set|with] check_part_changes->checkPartChanges->[bool];
    [get|set|with] cache_mesh_topology->cacheMeshTopology->[bool];
    [get|set|with] prefer_output_nodes->preferOutputNodes->[bool];
    [get|set|with] packed_prim_instancing_mode->packedPrimInstancingMode->[PackedPrimInstancingMode];
    [get+session] split_attr->splitAttrSH->[Result<String>];
);

#[derive(Debug, Clone)]
pub struct AttributeInfo(pub(crate) HAPI_AttributeInfo);

impl Default for AttributeInfo {
    fn default() -> Self {
        let mut inner = unsafe { HAPI_AttributeInfo_Create() };
        // FIXME: Uninitialized variable in Houdini 20.0.625
        inner.totalArrayElements = 0;
        Self(inner)
    }
}
wrap! {
  _impl_methods_ AttributeInfo HAPI_AttributeInfo[get]exists->exists->[bool];
  [get]original_owner->originalOwner->[AttributeOwner];
  [get|set|with]total_array_elements->totalArrayElements->[i64];
  [get|set|with]owner->owner->[AttributeOwner];
  [get|set|with]storage->storage->[StorageType];
  [get|set|with]tuple_size->tupleSize->[i32];
  [get|set|with]type_info->typeInfo->[AttributeTypeInfo];
  [get|set|with]count->count->[i32];
}

impl AttributeInfo {
    pub(crate) fn new(
        node: &HoudiniNode,
        part_id: i32,
        owner: AttributeOwner,
        name: &CStr,
    ) -> Result<Self> {
        Ok(Self(crate::ffi::get_attribute_info(
            node, part_id, owner, name,
        )?))
    }
}

/// [Documentation](https://www.sidefx.com/docs/hengine/struct_h_a_p_i___asset_info.html)
#[derive(Debug)]
pub struct AssetInfo(pub(crate) HAPI_AssetInfo, pub DebugIgnore<Session>);

impl AssetInfo {
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

/// [Documentation](https://www.sidefx.com/docs/hengine/struct_h_a_p_i___object_info.html)
#[derive(Debug)]
pub struct ObjectInfo<'session>(
    pub(crate) HAPI_ObjectInfo,
    pub DebugIgnore<&'session Session>,
);

impl ObjectInfo<'_> {
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
        self.node_id().to_node(&self.1)
    }
}

#[derive(Debug, Clone)]
/// [Documentation](https://www.sidefx.com/docs/hengine/struct_h_a_p_i___geo_info.html)
pub struct GeoInfo(pub(crate) HAPI_GeoInfo);

impl<'s> GeoInfo {
    get!(geo_type->type_->GeoType);
    get!(with_session name->nameSH->Result<String>);
    get!(node_id->nodeId->[handle: NodeHandle]);
    get!(is_editable->isEditable->bool);
    get!(is_templated->isTemplated->bool);
    get!(is_display_geo->isDisplayGeo->bool);
    get!(has_geo_changed->hasGeoChanged->bool);
    get!(has_material_changed->hasMaterialChanged->bool);
    get!(edge_group_count->edgeGroupCount->i32);
    get!(point_group_count->pointGroupCount->i32);
    get!(primitive_group_count->primitiveGroupCount->i32);
    get!(part_count->partCount->i32);

    pub fn from_node(node: &'s HoudiniNode) -> Result<Self> {
        GeoInfo::from_handle(node.handle, &node.session)
    }
    pub fn from_handle(handle: NodeHandle, session: &'s Session) -> Result<GeoInfo> {
        crate::ffi::get_geo_info(session, handle).map(GeoInfo)
    }
}

#[derive(Debug)]
pub struct PartInfo(pub(crate) HAPI_PartInfo);

wrap!(
    Default PartInfo [HAPI_PartInfo_Create => HAPI_PartInfo];
    [get] part_id->id->[i32];
    [get] attribute_counts->attributeCounts->[[i32; 4]];
    [get] has_changed->hasChanged->[bool];
    [get] is_instanced->isInstanced->[bool];
    [get+session] name->nameSH->[Result<String>];
    [get|set|with] part_type->type_->[PartType];
    [get|set|with] face_count->faceCount->[i32];
    [get|set|with] point_count->pointCount->[i32];
    [get|set|with] vertex_count->vertexCount->[i32];
    [get|set|with] instance_count->instanceCount->[i32];
    [get|set|with] instanced_part_count->instancedPartCount->[i32];
);

#[derive(Debug, Clone)]
pub struct TimelineOptions(pub(crate) HAPI_TimelineOptions);

wrap!(
    Default TimelineOptions [HAPI_TimelineOptions_Create => HAPI_TimelineOptions];
    [get|set|with] fps->fps->[f64];
    [get|set|with] start_time->startTime->[f64];
    [get|set|with] end_time->endTime->[f64];
);

#[derive(Debug, Clone)]
pub struct CurveInfo(pub(crate) HAPI_CurveInfo);

wrap!(
    Default CurveInfo [HAPI_CurveInfo_Create => HAPI_CurveInfo];
    [get|set|with] curve_type->curveType->[CurveType];
    [get|set|with] curve_count->curveCount->[i32];
    [get|set|with] vertex_count->vertexCount->[i32];
    [get|set|with] knot_count->knotCount->[i32];
    [get|set|with] periodic->isPeriodic->[bool];
    [get|set|with] rational->isRational->[bool];
    [get|set|with] closed->isClosed->[bool];
    [get|set|with] has_knots->hasKnots->[bool];
    [get|set|with] order->order->[i32];
);

#[derive(Debug, Clone)]
pub struct Viewport(pub(crate) HAPI_Viewport);

wrap!(
    Default Viewport [HAPI_Viewport_Create => HAPI_Viewport];
    [get|set|with] position->position->[[f32; 3]];
    [get|set|with] rotation->rotationQuaternion->[[f32; 4]];
    [get|set|with] offset->offset->[f32];
);

#[derive(Debug, Clone)]
/// [Documentation](https://www.sidefx.com/docs/hengine/struct_h_a_p_i___transform.html)
pub struct Transform(pub(crate) HAPI_Transform);

wrap!(
    Default Transform [HAPI_Transform_Create => HAPI_Transform];
    [get|set|with] position->position->[[f32;3]];
    [get|set|with] rotation->rotationQuaternion->[[f32;4]];
    [get|set|with] scale->scale->[[f32;3]];
    [get|set|with] shear->shear->[[f32;3]];
    [get|set|with] rst_order->rstOrder->[RSTOrder];
);

impl Transform {
    pub fn from_matrix(session: &Session, matrix: &[f32; 16], rst_order: RSTOrder) -> Result<Self> {
        crate::ffi::convert_matrix_to_quat(session, matrix, rst_order).map(Transform)
    }

    pub fn convert_to_matrix(&self, session: &Session) -> Result<[f32; 16]> {
        crate::ffi::convert_transform_quat_to_matrix(session, &self.0)
    }
}

#[derive(Debug, Clone)]
/// [Documentation](https://www.sidefx.com/docs/hengine/struct_h_a_p_i___transform_euler.html)
pub struct TransformEuler(pub(crate) HAPI_TransformEuler);

wrap!(
    Default TransformEuler [HAPI_TransformEuler_Create => HAPI_TransformEuler];
    [get|set|with] position->position->[[f32;3]];
    [get|set|with] rotation->rotationEuler->[[f32;3]];
    [get|set|with] scale->scale->[[f32;3]];
    [get|set|with] shear->shear->[[f32;3]];
    [get|set|with] roation_order->rotationOrder->[XYZOrder];
    [get|set|with] rst_order->rstOrder->[RSTOrder];
);

impl TransformEuler {
    pub fn convert_transform(
        &self,
        session: &Session,
        rst_order: RSTOrder,
        rot_order: XYZOrder,
    ) -> Result<Self> {
        crate::ffi::convert_transform(session, &self.0, rst_order, rot_order).map(TransformEuler)
    }

    pub fn from_matrix(
        session: &Session,
        matrix: &[f32; 16],
        rst_order: RSTOrder,
        rot_order: XYZOrder,
    ) -> Result<Self> {
        crate::ffi::convert_matrix_to_euler(session, matrix, rst_order, rot_order)
            .map(TransformEuler)
    }

    pub fn convert_to_matrix(&self, session: &Session) -> Result<[f32; 16]> {
        crate::ffi::convert_transform_euler_to_matrix(session, &self.0)
    }
}

#[derive(Debug, Clone)]
pub struct SessionSyncInfo(pub(crate) HAPI_SessionSyncInfo);

wrap!(
    Default SessionSyncInfo [HAPI_SessionSyncInfo_Create => HAPI_SessionSyncInfo];
    [get|set|with] cook_using_houdini_time->cookUsingHoudiniTime->[bool];
    [get|set|with] sync_viewport->syncViewport->[bool];
);

#[derive(Debug, Clone)]
pub struct BoxInfo(pub(crate) HAPI_BoxInfo);

// TODO: Why not impl Default?
fn _create_box_info() -> HAPI_BoxInfo {
    HAPI_BoxInfo {
        center: Default::default(),
        size: Default::default(),
        rotation: Default::default(),
    }
}

wrap!(
    Default BoxInfo [_create_box_info => HAPI_BoxInfo];
    [get|set|with] center->center->[[f32;3]];
    [get|set|with] rotation->rotation->[[f32;3]];
    [get|set|with] size->size->[[f32;3]];
);

#[derive(Debug, Clone)]
pub struct SphereInfo(pub(crate) HAPI_SphereInfo);

// TODO: Why not impl Default?
fn _create_sphere_info() -> HAPI_SphereInfo {
    HAPI_SphereInfo {
        center: Default::default(),
        radius: 0.0,
    }
}

wrap!(
    Default SphereInfo [_create_sphere_info => HAPI_SphereInfo];
    [get|set|with] center->center->[[f32;3]];
    [get|set|with] radius->radius->[f32];
);

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ImageInfo(pub(crate) HAPI_ImageInfo);

wrap!(
    Default ImageInfo [HAPI_ImageInfo_Create => HAPI_ImageInfo];
    [get|set|with] x_res->xRes->[i32];
    [get|set|with] y_res->yRes->[i32];
    [get|set|with] gamma->gamma->[f64];
    [get|set|with] data_format->dataFormat->[ImageDataFormat];
    [get|set|with] interleaved->interleaved->[bool];
    [get|set|with] packing->packing->[ImagePacking];
    [get+session] image_format->imageFileFormatNameSH->[Result<String>];
);

#[repr(C)]
#[derive(Debug, Clone)]
/// For parameter animation
pub struct KeyFrame {
    pub time: f32,
    pub value: f32,
    pub in_tangent: f32,
    pub out_tangent: f32,
}

#[derive(Debug, Clone)]
pub struct ImageFileFormat<'a>(
    pub(crate) HAPI_ImageFileFormat,
    pub(crate) DebugIgnore<&'a Session>,
);

impl ImageFileFormat<'_> {
    get!(name->nameSH->Result<String>);
    get!(description->descriptionSH->Result<String>);
    get!(extension->defaultExtensionSH->Result<String>);
}

#[derive(Debug, Clone)]
pub struct VolumeInfo(pub(crate) HAPI_VolumeInfo);

wrap!(
    impl VolumeInfo => HAPI_VolumeInfo;
    [get+session] name->nameSH->[Result<String>];
    [get] volume_type->type_->[VolumeType];
    [get|set|with] x_length->xLength->[i32];
    [get|set|with] y_length->yLength->[i32];
    [get|set|with] z_length->zLength->[i32];
    [get|set|with] min_x->minX->[i32];
    [get|set|with] min_y->minY->[i32];
    [get|set|with] min_z->minZ->[i32];
    [get|set|with] tuple_size->tupleSize->[i32];
    [get|set|with] storage->storage->[StorageType];
    [get|set|with] tile_size->tileSize->[i32];
    [get|set|with] has_taper->hasTaper->[bool];
    [get|set|with] x_taper->xTaper->[f32];
    [get|set|with] y_taper->yTaper->[f32];
);

impl VolumeInfo {
    fn transform(&self) -> Transform {
        Transform(self.0.transform)
    }
    fn set_transform(&mut self, transform: Transform) {
        self.0.transform = transform.0
    }
    fn with_transform(mut self, transform: Transform) -> Self {
        self.0.transform = transform.0;
        self
    }
}

#[derive(Debug, Clone)]
pub struct VolumeTileInfo(pub(crate) HAPI_VolumeTileInfo);

wrap!(
    impl VolumeTileInfo => HAPI_VolumeTileInfo;
    [get|set|with] min_x->minX->[i32];
    [get|set|with] min_y->minY->[i32];
    [get|set|with] min_z->minZ->[i32];
    [get] is_valid->isValid->[bool];
);

#[derive(Debug, Clone)]
pub struct VolumeVisualInfo(pub(crate) HAPI_VolumeVisualInfo);

wrap!(
    impl VolumeVisualInfo => HAPI_VolumeVisualInfo;
    [get|set|with] visual_type->type_->[VolumeVisualType];
    [get|set|with] iso->iso->[f32];
    [get|set|with] density->density->[f32];
);

#[derive(Debug, Clone)]
pub struct InputCurveInfo(pub(crate) HAPI_InputCurveInfo);

wrap!(
    Default InputCurveInfo [HAPI_InputCurveInfo_Create => HAPI_InputCurveInfo];
    [get|set|with] curve_type->curveType->[CurveType];
    [get|set|with] order->order->[i32];
    [get|set|with] closed->closed->[bool];
    [get|set|with] reverse->reverse->[bool];
    [get|set|with] input_method->inputMethod->[InputCurveMethod];
    [get|set|with] breakpoint_parameterization->breakpointParameterization->[InputCurveParameterization];
);

#[derive(Debug, Copy, Clone)]
pub struct PDGEventInfo(pub(crate) HAPI_PDG_EventInfo);

impl PDGEventInfo {
    get!(node_id->nodeId->[handle: NodeHandle]);
    get!(workitem_id->workItemId->[handle: WorkItemId]);
    get!(dependency_id->dependencyId->i32);
    get!(with_session message->msgSH->Result<String>);
    pub fn current_state(&self) -> PdgWorkItemState {
        unsafe { std::mem::transmute::<i32, PdgWorkItemState>(self.0.currentState) }
    }
    pub fn last_state(&self) -> PdgWorkItemState {
        unsafe { std::mem::transmute::<i32, PdgWorkItemState>(self.0.lastState) }
    }
    pub fn event_type(&self) -> PdgEventType {
        unsafe { std::mem::transmute::<i32, PdgEventType>(self.0.eventType) }
    }
}

#[derive(Debug)]
pub struct PDGWorkItemOutputFile<'session>(
    pub(crate) HAPI_PDG_WorkItemOutputFile,
    pub(crate) DebugIgnore<&'session Session>,
);

impl PDGWorkItemOutputFile<'_> {
    get!(path->filePathSH->Result<String>);
    get!(tag->tagSH->Result<String>);
    get!(sha->hash->i64);
}

pub struct PDGWorkItemInfo(pub(crate) HAPI_PDG_WorkItemInfo);

wrap! {
    impl PDGWorkItemInfo => HAPI_PDG_WorkItemInfo;
    [get] index->index->[i32];
    [get] output_file_count->outputFileCount->[i32];
    [get+session] name->nameSH->[Result<String>];
}
