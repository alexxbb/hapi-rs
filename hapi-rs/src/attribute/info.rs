use crate::ffi;

mod var_1 {
    use super::*;
    use crate::ffi::*;
    use crate::{builder, wrap_ffi};

    #[derive(Debug)]
    pub struct AttributeInfo {
        pub(crate) inner: ffi::HAPI_AttributeInfo,
    }

    builder!(
        @name: AttributeInfoBuilder,
        @ffi: ffi::HAPI_AttributeInfo,
        @default: { ffi::HAPI_AttributeInfo_Create() },
        @result: AttributeInfo,
        @setters: {
            owner->owner: AttributeOwner,
            storage->storage: StorageType,
            count->count: i32,
            tuple_size->tupleSize: i32,
        }
    );
    wrap_ffi!(AttributeInfo, self,
        owner->AttributeOwner { self.inner.owner },
        storage->StorageType { self.inner.storage },
        exists->bool { self.inner.exists == 1 },
        count->i32 { self.inner.count },
        tuple_size->i32 { self.inner.tupleSize },
        type_info->AttributeTypeInfo { self.inner.typeInfo },
        total_array_elements->i64 {self.inner.totalArrayElements }
    );
}
