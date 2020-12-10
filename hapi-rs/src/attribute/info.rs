use crate::ffi;

// mod var_1 {
//     use super::*;
//     use crate::ffi::*;
//
//     #[derive(Debug)]
//     pub struct AttributeInfo {
//         pub(crate) inner: ffi::HAPI_AttributeInfo,
//     }
//
//     builder!(
//         @name: AttributeInfoBuilder,
//         @ffi: ffi::HAPI_AttributeInfo,
//         @default: { ffi::HAPI_AttributeInfo_Create() },
//         @result: AttributeInfo,
//     );
//     impl AttributeInfo {
//         fn_getter!(owner, owner, AttributeOwner);
//         fn_getter!(storage, storage, StorageType);
//         fn_getter!(exists, exists, bool);
//         fn_getter!(count, count, i32);
//         fn_getter!(tuple_size, tupleSize, i32);
//         fn_getter!(type_info, typeInfo, AttributeTypeInfo);
//         fn_getter!(total_array_elements, totalArrayElements, i64);
//     }
// }
