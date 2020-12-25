use crate::ffi::raw::{
    AttributeOwner, AttributeTypeInfo, HAPI_AttributeInfo, HAPI_AttributeInfo_Create, StorageType,
};
#[derive(Debug)]
pub struct AttributeInfo {
    pub(crate) inner: HAPI_AttributeInfo,
}

// wrap_ffi!(
//         @object: AttributeInfo
//         @builder: AttributeInfoBuilder
//         @default: [HAPI_AttributeInfo_Create => HAPI_AttributeInfo]
//         methods:
//             owner->owner->[AttributeOwner];
//             storage->storage->[StorageType];
//             exists->exists->[bool];
//             count->count->[i32];
//             tuple_size->tupleSize->[i32];
//             type_info->typeInfo->[AttributeTypeInfo];
//             total_array_elements->totalArrayElements->[i64];
//
// );