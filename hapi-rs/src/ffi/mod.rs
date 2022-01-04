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
        AttributeOwner, AttributeTypeInfo, CacheProperty, ChoiceListType, CurveOrders, CurveType,
        EnvIntType, GeoType, GroupType, HapiResult, HeightFieldSampling, ImageDataFormat,
        ImagePacking, InputType, License, PackedPrimInstancingMode, ParmType, PartType,
        PdgEventType, PdgState, PdgWorkitemState, Permissions, PresetType, PrmScriptType, RSTOrder,
        RampType, SessionEnvIntType, SessionType, State, StatusType, StatusVerbosity, StorageType,
        TransformComponent, VolumeType, VolumeVisualType, XYZOrder,
    };
}
