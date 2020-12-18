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
pub enum ParmValue<T> {
    Single(T),
    Tuple2((T, T)),
    Tuple3((T, T, T)),
    Tuple4((T, T, T, T)),
    Array(Vec<T>),
}

impl<T> From<ParmValue<T>> for Vec<T> {
    fn from(v: ParmValue<T>) -> Self {
        let mut vals = Vec::with_capacity(4);

        match v {
            ParmValue::Single(v) => vals.push(v),
            ParmValue::Tuple2((v1, v2)) => {
                vals.push(v1);
                vals.push(v2);
            }
            ParmValue::Tuple3((v1, v2, v3)) => {
                vals.push(v1);
                vals.push(v2);
                vals.push(v3);
            }
            ParmValue::Tuple4((v1, v2, v3, v4)) => {
                vals.push(v1);
                vals.push(v2);
                vals.push(v3);
                vals.push(v4);
            }
            ParmValue::Array(v) => vals = v,
        };
        vals
    }
}


impl<'s, T: 's> From<&'s ParmValue<T>> for Vec<&'s str> where T: AsRef<str> {
    fn from(v: &'s ParmValue<T>) -> Self {
        let mut vals = Vec::with_capacity(4);

        match v {
            ParmValue::Single(v) => vals.push(v.as_ref()),
            ParmValue::Tuple2((v1, v2)) => {
                vals.push(v1.as_ref());
                vals.push(v2.as_ref());
            }
            ParmValue::Tuple3((v1, v2, v3)) => {
                vals.push(v1.as_ref());
                vals.push(v2.as_ref());
                vals.push(v3.as_ref());
            }
            ParmValue::Tuple4((v1, v2, v3, v4)) => {
                vals.push(v1.as_ref());
                vals.push(v2.as_ref());
                vals.push(v3.as_ref());
                vals.push(v4.as_ref());
            }
            ParmValue::Array(v) => vals = v.iter().map(|vv| vv.as_ref()).collect(),
        };
        vals
    }
}

impl<T> From<T> for ParmValue<T> {
    fn from(v: T) -> Self {
        Self::Single(v)
    }
}

impl<'a, T> From<[&'a T; 2]> for ParmValue<&'a T> {
    fn from(v: [&'a T; 2]) -> Self {
        Self::Tuple2((v[0], v[1]))
    }
}

// impl<'a, T> From<[&'a T; 3]> for ParmValue<&'a T> {
//     fn from(v: [&'a T; 3]) -> Self {
//         Self::Tuple3((v[0], v[1], v[2]))
//     }
// }

impl<'a, T: Clone> From<[T; 3]> for ParmValue<T> {
    fn from(v: [T; 3]) -> Self {
        Self::Tuple3((v[0].clone(), v[1].clone(), v[2].clone()))
    }
}

impl<'a, T> From<[&'a T; 4]> for ParmValue<&'a T> {
    fn from(v: [&'a T; 4]) -> Self {
        Self::Tuple4((v[0], v[1], v[2], v[3]))
    }
}

#[derive(Debug)]
pub enum Parameter<'session> {
    Float(FloatParameter<'session>),
    Int(IntParameter<'session>),
    String(StringParameter<'session>),
    Other,
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
            ParmType::Int | ParmType::Button => Parameter::Int(IntParameter { wrap: base }),
            ParmType::Float | ParmType::Color => Parameter::Float(FloatParameter { wrap: base }),
            ParmType::String
            | ParmType::PathFile
            | ParmType::PathFileDir
            | ParmType::PathFileGeo
            | ParmType::PathFileImage => Parameter::String(StringParameter { wrap: base }),
            _ => Parameter::Other,
        }
    }
    pub fn info(&self) -> &ParmInfo {
        match self {
            Parameter::Float(p) => &p.wrap.info,
            Parameter::Int(p) => &p.wrap.info,
            Parameter::String(p) => &p.wrap.info,
            Parameter::Other => unreachable!()
        }
    }
}

impl<'s> ParmBaseTrait<'s> for FloatParameter<'s> {
    type ReturnType = f32;

    fn wrap(&self) -> &ParmNodeWrap<'s> {
        &self.wrap
    }

    fn array_index(&self) -> i32 {
        self.wrap.info.float_values_index()
    }

    fn values_array(&self) -> Result<Vec<Self::ReturnType>> {
        let start = self.wrap.info.float_values_index();
        let count = self.wrap.info.size();
        super::values::get_float_values(
            &self.wrap.node.handle,
            &self.wrap.node.session,
            start,
            count,
        )
    }
}

impl<'s> FloatParameter<'s> {
    pub fn set_value<T>(&self, val: T) -> Result<()>
        where
            T: Into<ParmValue<f32>>,
    {
        let mut vals: Vec<f32> = val.into().into();
        super::values::set_float_values(&self.wrap.node.handle,
                                        &self.wrap.node.session,
                                        self.wrap.info.float_values_index(),
                                        self.wrap.info.size(), &vals)
    }
}

impl<'s> ParmBaseTrait<'s> for IntParameter<'s> {
    type ReturnType = i32;

    fn wrap(&self) -> &ParmNodeWrap<'s> {
        &self.wrap
    }

    fn array_index(&self) -> i32 {
        self.wrap.info.int_values_index()
    }

    fn values_array(&self) -> Result<Vec<Self::ReturnType>> {
        let start = self.wrap.info.int_values_index();
        let count = self.wrap.info.size();
        super::values::get_int_values(
            &self.wrap.node.handle,
            &self.wrap.node.session,
            start,
            count,
        )
    }
}

impl<'s> IntParameter<'s> {
    pub fn set_value<T>(&self, val: T) -> Result<()>
        where
            T: Into<ParmValue<i32>>,
    {
        let mut vals: Vec<i32> = val.into().into();
        super::values::set_int_values(&self.wrap.node.handle,
                                      &self.wrap.node.session,
                                      self.wrap.info.int_values_index(),
                                      self.wrap.info.size(), &vals)
    }
}

impl<'s> ParmBaseTrait<'s> for StringParameter<'s> {
    type ReturnType = String;

    fn wrap(&self) -> &ParmNodeWrap<'s> {
        &self.wrap
    }

    fn array_index(&self) -> i32 {
        self.wrap.info.string_values_index()
    }

    fn values_array(&self) -> Result<Vec<Self::ReturnType>> {
        let start = self.array_index();
        let count = self.wrap.info.size();
        super::values::get_string_values(
            &self.wrap.node.handle,
            &self.wrap.node.session,
            start,
            count,
        )
    }
}

impl<'s> StringParameter<'s> {
    pub fn set_value<T, R>(&self, val: T) -> Result<()>
        where
            R: AsRef<str>,
            T: Into<ParmValue<R>>,
    {

        let pv = val.into();
        let vals: Vec<&str> = (&pv).into();
        use std::result::Result;
        let c_str = vals.iter().map(|s| CString::new(*s))
            .collect::<Result<Vec<CString>, _>>()?;
        let c_str:Vec<&CStr> = c_str.iter().map(|c|c.as_ref()).collect();
        super::values::set_string_values(&self.wrap.node.handle, &self.wrap.info.id(), &self.wrap.node.session,
                                         &c_str)
    }
}

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
