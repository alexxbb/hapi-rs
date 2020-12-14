use crate::{
    errors::Result,
    ffi,
    ffi::{
        ChoiceListType, HAPI_GetParmInfo, HAPI_NodeId, HAPI_ParmId, HAPI_ParmInfo,
        HAPI_ParmInfo_Create, NodeFlags, NodeType, ParmType, Permissions, PrmScriptType, RampType,
    },
    node::{HoudiniNode, NodeHandle, NodeInfo},
    session::Session,
};
use std::ffi::{CString, CStr};
use std::fmt::Formatter;
use std::mem::MaybeUninit;
use std::rc::Rc;

impl Default for HAPI_ParmInfo {
    fn default() -> Self {
        unimplemented!()
    }
}

pub(crate) fn read_info_from_handle(
    session: &Session,
    node_handle: &NodeHandle,
    parm_handle: &ParmHandle,
    info: &mut ParmInfo,
) -> Result<()> {
    unsafe {
        HAPI_GetParmInfo(
            session.ptr(),
            node_handle.0,
            parm_handle.0,
            &mut info.inner as *mut _,
        )
        .result_with_session(|| session.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ParmHandle(pub HAPI_ParmId);

impl ParmHandle {
    pub fn from_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        let id = unsafe {
            let name = CString::new(name)?;
            let mut id = MaybeUninit::uninit();
            ffi::HAPI_GetParmIdFromName(
                node.session.ptr(),
                node.handle.0,
                name.as_ptr(),
                id.as_mut_ptr(),
            )
            .result_with_session(|| node.session.clone())?;
            id.assume_init()
        };
        Ok(ParmHandle(id))
    }
    pub fn info<'s>(&self, node: &'s HoudiniNode) -> Result<ParmInfo<'s>> {
        let mut info = ParmInfo {
            inner: unsafe { HAPI_ParmInfo_Create() },
            session: &node.session,
        };
        read_info_from_handle(&node.session, &node.handle, self, &mut info)?;
        Ok(info)
    }
}

pub trait ParmValueType{}

impl ParmValueType for f32{}

pub struct Parameter<'session> {
    pub info: ParmInfo<'session>,
    session: &'session Session,
    name: Option<CString>,
    node: Rc<NodeInfo>,
}

impl std::fmt::Debug for Parameter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parameter[{name} of type {type}]",
               name=self.info.name().unwrap(),
                type=self.info.parm_type().as_ref())
    }
}

pub enum ParmValue<T> {
    Value(T),
    Tuple2((T, T)),
    Tuple3((T, T, T)),
    Tuple4((T, T, T, T)),
    Array(Vec<T>),
    // Int(i32),
    // Float(f32),
    // String(String),
    // Tuple2((T, T)),
    // Tuple3((T, T, T)),
    // Tuple4((T, T, T, T)),
    // Array(Vec<T>),
    // Node(HoudiniNode),
}


impl<'session> Parameter<'session> {
    //TODO: revisit. maybe borrow NodeInfo instead of Rc?
    // And maybe HoudiniNode instead of Session?
    // Do we need HoudiniNode for cooking when setting values?
    pub(crate) fn new(
        node: Rc<NodeInfo>,
        info: HAPI_ParmInfo,
        session: &'session Session,
        name: Option<CString>,
    ) -> Parameter<'session> {
        let info = ParmInfo {
            inner: info,
            session,
        };
        Self {
            info,
            session,
            node,
            name,
        }
    }

    pub fn name(&self) -> Result<String> {
        match self.name.as_ref() {
            None => {self.info.name()}
            Some(n) => {
                Ok(n.to_string_lossy().to_string())
            }
        }
    }

    pub(crate) fn all_parm_float_values(handle: &NodeHandle, session: &Session, index: i32, count: i32) -> Result<Vec<f32>> {
        let mut values = vec![0.;count as usize];
        unsafe {
            ffi::HAPI_GetParmFloatValues(session.ptr(),
                                         handle.0,
                                         values.as_mut_ptr(),
                                         index,
                                         count)
                .result_with_session(||session.clone())?;

        }
        Ok(values)
    }

    unsafe fn get_float_value(&self) -> Result<f32> {
        let mut val = MaybeUninit::uninit();
        let mut tmp;
        let name = match &self.name  {
            None => {
                tmp = self.info.name_cstr()?;
                tmp.as_c_str()
            }
            Some(n) => n.as_c_str()
        };
        ffi::HAPI_GetParmFloatValue(
            self.session.ptr(),
            self.node.inner.id,
            name.as_ptr(),
            0, // TODO: This may be a parm on N components
            val.as_mut_ptr(),
        )
        .result_with_session(|| self.session.clone())?;
        Ok(val.assume_init())
    }

    pub fn get_value<T>(&self) -> Result<ParmValue<T>> {
        let v = match self.info.parm_type() {
            ParmType::Int => {
                unimplemented!()
            },
            ParmType::Float => {
                let index = self.info.float_values_index();
                let values = Parameter::all_parm_float_values(
                    &self.node.node_handle(),
                    &self.session,
                    index,
                    self.info.size())?;
                match self.info.size() {
                    1 => ParmValue::Value(values[0]),
                    2 => ParmValue::Tuple2((values[0], values[1])),
                    3 => ParmValue::Tuple3((values[0], values[1], values[2])),
                    4 => ParmValue::Tuple4((values[0], values[1], values[2], values[3])),
                    _ => ParmValue::Array(values)

                }
            },
            ParmType::String => unimplemented!(),
            _ => unimplemented!() // Logic error?
        };
        Ok(v)
    }
}

pub struct ParmInfo<'session> {
    pub(crate) inner: HAPI_ParmInfo,
    pub(crate) session: &'session Session,
}

macro_rules! _get_str {
    ($m:ident->$f:ident) => {
        pub fn $m(&self) -> Result<String> {
            self.session.get_string(self.inner.$f)
        }
    };
}

impl<'session> ParmInfo<'session> {
    pub(crate) fn from_name(name: &CStr, node: &'session HoudiniNode) -> Result<Self> {
        let info = unsafe {
            let mut info = MaybeUninit::uninit();
            ffi::HAPI_GetParmInfoFromName(
                node.session.ptr(),
                node.handle.0,
                name.as_ptr(),
                info.as_mut_ptr(),
            )
                .result_with_session(|| node.session.clone())?;
            info.assume_init()
        };

        Ok(ParmInfo {
            inner: info,
            session: &node.session,
        })
    }

    pub(crate) fn name_cstr(&self) -> Result<CString> {
        crate::stringhandle::get_cstring(self.inner.nameSH, &self.session)
    }

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
    _get_str!(type_info->typeInfoSH);
    _get_str!(name->nameSH);
    _get_str!(label->labelSH);
    _get_str!(template_name->templateNameSH);
    _get_str!(help->helpSH);
    _get_str!(visibility_condition->visibilityConditionSH);
    _get_str!(disabled_condition->disabledConditionSH);
}
