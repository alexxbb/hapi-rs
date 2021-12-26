#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#[allow(clippy::all)]
mod bindings;
mod functions;
pub mod structs;

pub(crate) mod raw {
    pub use super::bindings::*;
}
pub(crate) use functions::*;
pub use structs::*;

pub mod enums {
    pub use super::bindings::{
        License,
        StatusType,
        StatusVerbosity,
        HapiResult,
        SessionType,
        State,
        PackedPrimInstancingMode,
        Permissions,
        RampType,
        ParmType,
        PrmScriptType,
        ChoiceListType,
        PresetType,
        GroupType,
        AttributeOwner,
        CurveType,
        VolumeType,
        VolumeVisualType,
        StorageType,
        AttributeTypeInfo,
        GeoType,
        PartType,
        InputType,
        CurveOrders,
        TransformComponent,
        RSTOrder,
        XYZOrder,
        ImageDataFormat,
        ImagePacking,
        EnvIntType,
        SessionEnvIntType,
        CacheProperty,
        HeightFieldSampling,
        PdgState,
        PdgEventType,
        PdgWorkitemState
    };
}
