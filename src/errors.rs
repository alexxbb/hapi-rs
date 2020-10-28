use crate::ffi;


pub enum HAPI_Error {
    SUCCESS(ffi::HAPI_Result),
    FAILURE(ffi::HAPI_Result),
    ALREADY_INITIALIZED(ffi::HAPI_Result),
    NOT_INITIALIZED(ffi::HAPI_Result),
    CANT_LOADFILE(ffi::HAPI_Result),
    PARM_SET_FAILED(ffi::HAPI_Result),
    INVALID_ARGUMENT(ffi::HAPI_Result),
    CANT_LOAD_GEO(ffi::HAPI_Result),
    CANT_GENERATE_PRESET(ffi::HAPI_Result),
    CANT_LOAD_PRESET(ffi::HAPI_Result),
    ASSET_DEF_ALREADY_LOADED(ffi::HAPI_Result),
    NO_LICENSE_FOUND(ffi::HAPI_Result),
    DISALLOWED_NC_LICENSE_FOUND(ffi::HAPI_Result),
    DISALLOWED_NC_ASSET_WITH_C_LICENSE(ffi::HAPI_Result),
    DISALLOWED_NC_ASSET_WITH_LC_LICENSE(ffi::HAPI_Result),
    DISALLOWED_LC_ASSET_WITH_C_LICENSE(ffi::HAPI_Result),
    DISALLOWED_HENGINEINDIE_W_3PARTY_PLUGIN(ffi::HAPI_Result),
    ASSET_INVALID(ffi::HAPI_Result),
    NODE_INVALID(ffi::HAPI_Result),
    USER_INTERRUPTED(ffi::HAPI_Result),
    INVALID_SESSION(ffi::HAPI_Result)
}

impl From<ffi::HAPI_Result> for HAPI_Error {
    fn from(e: ffi::HAPI_Result) -> HAPI_Error {
        match e {
            ffi::HAPI_Result_HAPI_RESULT_USER_INTERRUPTED =>
                HAPI_Error::USER_INTERRUPTED(ffi::HAPI_Result_HAPI_RESULT_USER_INTERRUPTED),
            ffi::HAPI_Result_HAPI_RESULT_SUCCESS =>
                HAPI_Error::SUCCESS(ffi::HAPI_Result_HAPI_RESULT_SUCCESS),
            ffi::HAPI_Result_HAPI_RESULT_FAILURE =>
                HAPI_Error::FAILURE(ffi::HAPI_Result_HAPI_RESULT_FAILURE),
        }
    }
}