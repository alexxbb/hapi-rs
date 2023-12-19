#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#[allow(clippy::all)]
mod bindings;
mod functions;
pub mod structs;

pub mod raw {
    pub use super::bindings::*;
}
pub(crate) use functions::*;
pub use structs::*;

/// All Engine API enums are here
///
/// Refer to [Houdini documentation](https://www.sidefx.com/docs/hengine/_h_a_p_i___common_8h.html#ab8e5b8743050848e96767af662b23f1d)
pub mod enums {
    pub use super::bindings::{
        AttributeOwner, AttributeTypeInfo, CacheProperty, ChoiceListType, CurveOrders, CurveType,
        EnvIntType, GeoType, GroupType, HapiResult, HeightFieldSampling, ImageDataFormat,
        ImagePacking, InputType, License, PackedPrimInstancingMode, ParmType, PartType,
        PdgEventType, PdgState, PdgWorkItemState, Permissions, PresetType, PrmScriptType, RSTOrder,
        RampType, SessionEnvIntType, SessionType, State, StatusType, StatusVerbosity, StorageType,
        TransformComponent, VolumeType, VolumeVisualType, XYZOrder,
    };
}
