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
use super::traits::*;
use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::fmt::Formatter;
use std::mem::MaybeUninit;
use std::rc::Rc;

use log::warn;

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
            name: None,
        };
        unsafe {
            HAPI_GetParmInfo(
                node.session.ptr(),
                node.handle.0,
                info.inner.id,
                &mut info.inner as *mut _,
            )
                .result_with_session(|| node.session.clone())?
        }
        Ok(info)
    }
}

// TODO: Should be private
pub struct ParmNodeWrap<'session> {
    pub info: ParmInfo<'session>,
    pub node: &'session HoudiniNode,
}

#[derive(Debug)]
pub struct BaseParameter<'session> {
    wrap: ParmNodeWrap<'session>,
}

#[derive(Debug)]
pub struct FloatParameter<'session> {
    wrap: ParmNodeWrap<'session>,
}

#[derive(Debug)]
pub struct IntParameter<'session> {
    wrap: ParmNodeWrap<'session>,
}

#[derive(Debug)]
pub struct StringParameter<'session> {
    wrap: ParmNodeWrap<'session>,
}

#[derive(Debug)]
pub enum Parameter<'session> {
    Float(FloatParameter<'session>),
    Int(IntParameter<'session>),
    String(StringParameter<'session>),
    Other(BaseParameter<'session>),
}


impl<'node> Parameter<'node> {
    pub(crate) fn new(
        node: &'node HoudiniNode,
        info: ParmInfo<'node>,
    ) -> Parameter<'node> {
        let base = ParmNodeWrap {
            info,
            node,
        };
        eprintln!("{:?}->{:?}", &base.info.parm_type(), &base.info.script_type());
        match base.info.parm_type() {
            ParmType::Int
            | ParmType::Button
            | ParmType::Toggle
            | ParmType::Folder
            | ParmType::Folderlist
            => Parameter::Int(IntParameter { wrap: base }),
            ParmType::Float | ParmType::Color => Parameter::Float(FloatParameter { wrap: base }),
            ParmType::String
            | ParmType::Node
            | ParmType::PathFile
            | ParmType::PathFileDir
            | ParmType::PathFileGeo
            | ParmType::PathFileImage => Parameter::String(StringParameter { wrap: base }),
            _ => unreachable!()
        }
    }
    pub fn info(&self) -> &ParmInfo {
        match self {
            Parameter::Float(p) => &p.wrap.info,
            Parameter::Int(p) => &p.wrap.info,
            Parameter::String(p) => &p.wrap.info,
            Parameter::Other(p) => &p.wrap.info
        }
    }

    pub fn name(&self) -> Result<Cow<'_, str>> {
        match self {
            Parameter::Float(p) => p.name(),
            Parameter::Int(p) => p.name(),
            Parameter::String(p) => p.name(),
            Parameter::Other(_) => unimplemented!() // TODO. BaseParameter is missing name()
        }
    }
}

impl<'s> ParmBaseTrait<'s> for FloatParameter<'s> {
    type ValueType = f32;

    fn wrap(&self) -> &ParmNodeWrap<'s> {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>> {
        let start = self.wrap.info.float_values_index();
        let count = self.wrap.info.size();
        super::values::get_float_values(
            &self.wrap.node.handle,
            &self.wrap.node.session,
            start,
            count,
        )
    }

    fn set_value<T>(&self, val: T) -> Result<()> where T: AsRef<[Self::ValueType]> {
        super::values::set_float_values(&self.wrap.node.handle,
                                        &self.wrap.node.session,
                                        self.wrap.info.float_values_index(),
                                        self.wrap.info.size(), val.as_ref())
    }
}


impl<'s> ParmBaseTrait<'s> for IntParameter<'s> {
    type ValueType = i32;

    fn wrap(&self) -> &ParmNodeWrap<'s> {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>> {
        let start = self.wrap.info.int_values_index();
        let count = self.wrap.info.size();
        super::values::get_int_values(
            &self.wrap.node.handle,
            &self.wrap.node.session,
            start,
            count,
        )
    }

    fn set_value<T>(&self, val: T) -> Result<()> where T: AsRef<[Self::ValueType]> {
        let start = self.wrap.info.int_values_index();
        let count = self.wrap.info.size();
        super::values::set_int_values(&self.wrap.node.handle,
                                      &self.wrap.node.session,
                                      start,
                                      count, val.as_ref())
    }
}

impl<'s> ParmBaseTrait<'s> for StringParameter<'s> {
    type ValueType = String;

    fn wrap(&self) -> &ParmNodeWrap<'s> {
        &self.wrap
    }


    fn get_value(&self) -> Result<Vec<Self::ValueType>> {
        let start = self.wrap.info.string_values_index();
        let count = self.wrap.info.size();
        super::values::get_string_values(
            &self.wrap.node.handle,
            &self.wrap.node.session,
            start,
            count,
        )
    }

    // TODO Maybe take it out of the trait? AsRef makes it an extra String copy. Consider ToOwned?
    fn set_value<T>(&self, val: T) -> Result<()> where T: AsRef<[Self::ValueType]> {
        let start = self.wrap.info.string_values_index();
        let count = self.wrap.info.size();
        let c_str: Vec<CString> = val.as_ref().into_iter().map(|s| unsafe { CString::new(s.clone()).expect("Null string") }).collect();
        super::values::set_string_values(&self.wrap.node.handle,
                                         &self.wrap.info.id(),
                                         &self.wrap.node.session,
                                         &c_str)
    }
}

impl<'s> StringParameter<'s> {}

impl<'node> ParmNodeWrap<'node> {
    pub(crate) fn c_name(&self) -> Result<Cow<CString>> {
        match self.info.name.as_ref() {
            None => Ok(Cow::Owned(self.info.name_cstr()?)),
            Some(n) => Ok(Cow::Borrowed(n)),
        }
    }
}

impl std::fmt::Debug for ParmNodeWrap<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parameter[{name} of type {type}]",
               name=self.info.name().unwrap(),
                type=self.info.parm_type().as_ref())
    }
}

#[derive(Debug)]
pub struct ParmInfo<'session> {
    pub(crate) inner: HAPI_ParmInfo,
    pub(crate) session: &'session Session,
    pub(crate) name: Option<CString>,
}

macro_rules! _get_str {
    ($m:ident->$f:ident) => {
        pub fn $m(&self) -> Result<String> {
            self.session.get_string(self.inner.$f)
        }
    };
}

impl<'session> ParmInfo<'session> {
    pub(crate) fn from_name(name: CString, node: &'session HoudiniNode) -> Result<Self> {
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
            name: Some(name),
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
