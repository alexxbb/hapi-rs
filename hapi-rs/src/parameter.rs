use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::fmt::Formatter;
use std::mem::MaybeUninit;
use std::rc::Rc;

use log::warn;

pub use crate::{
    errors::Result,
    ffi::raw::{
        ChoiceListType, HAPI_ParmId, NodeFlags, NodeType, ParmType, Permissions, PrmScriptType,
        RampType,
    },
    ffi::{NodeInfo, ParmChoiceInfo, ParmInfo},
    node::{HoudiniNode, NodeHandle},
    session::Session,
};

pub trait ParmBaseTrait<'s> {
    type ValueType;

    fn c_name(&'s self) -> Result<Cow<'s, CString>> {
        let info = &self.wrap().info;
        match info.name.as_ref() {
            None => Ok(Cow::Owned(info.name_cstr()?)),
            Some(n) => Ok(Cow::Borrowed(n)),
        }
    }

    fn name(&'s self) -> Result<Cow<'s, str>> {
        match self.c_name()? {
            Cow::Borrowed(s) => unsafe {
                let bytes = s.as_bytes();
                Ok(Cow::Borrowed(std::str::from_utf8_unchecked(
                    &bytes[..bytes.len() - 1],
                )))
            },
            Cow::Owned(s) => Ok(Cow::Owned(s.into_string().unwrap())),
        }
    }
    fn is_menu(&self) -> bool {
        !matches!(self.wrap().info.choice_list_type(), ChoiceListType::None)
    }
    fn wrap(&self) -> &ParmNodeWrap<'s>;
    // TODO find a way to make it private
    fn info(&self) -> &ParmInfo<'s> {
        &self.wrap().info
    }
    fn menu_items(&'s self) -> Option<Result<Vec<ParmChoiceInfo<'s>>>> {
        if !self.is_menu() {
            return None;
        }
        let wrap = self.wrap();
        let parms = crate::ffi::get_parm_choice_list(
            &wrap.node,
            wrap.info.choice_index(),
            wrap.info.choice_count(),
        );
        let parms = parms.map(|v| {
            v.into_iter()
                .map(|p| ParmChoiceInfo {
                    inner: p,
                    session: &wrap.node.session,
                })
                .collect::<Vec<ParmChoiceInfo<'_>>>()
        });
        Some(parms)
    }
    fn expression(&'s self, index: i32) -> Result<String> {
        let wrap = self.wrap();
        crate::ffi::get_parm_expression(&wrap.node, self.c_name()?.as_c_str(), index)
    }

    fn set_expression(&'s self, value: &str, index: i32) -> Result<()> {
        let wrap = self.wrap();
        let value = CString::new(value)?;
        crate::ffi::set_parm_expression(&wrap.node, &wrap.info.id(), &value, index)
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>>;
    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>;
}

#[derive(Debug, Clone)]
pub struct ParmHandle(pub HAPI_ParmId);

impl ParmHandle {
    pub fn from_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        let name = CString::new(name)?;
        let id = crate::ffi::get_parm_id_from_name(&name, node)?;
        Ok(ParmHandle(id))
    }
    pub fn info<'s>(&self, node: &'s HoudiniNode) -> Result<ParmInfo<'s>> {
        let info = crate::ffi::get_parm_info(node, &self)?;
        Ok(ParmInfo {
            inner: info,
            session: &node.session,
            name: None,
        })
    }
}

impl<'node> ParmInfo<'node> {
    pub fn from_parm_name(name: &str, node: &'node HoudiniNode) -> Result<Self> {
        let name = CString::new(name)?;
        let info = crate::ffi::get_parm_info_from_name(&node, &name);
        info.map(|info| ParmInfo {
            inner: info,
            session: &node.session,
            name: Some(name),
        })
    }

    pub(crate) fn from_ffi(
        info: crate::ffi::raw::HAPI_ParmInfo,
        node: &'node HoudiniNode,
    ) -> Result<Self> {
        Ok(ParmInfo {
            inner: info,
            session: &node.session,
            name: None,
        })
    }
}

// TODO: Should be private
pub struct ParmNodeWrap<'session> {
    pub info: ParmInfo<'session>,
    pub node: &'session HoudiniNode,
}

#[derive(Debug)]
pub struct BaseParameter<'session> {
    pub(crate) wrap: ParmNodeWrap<'session>,
}

#[derive(Debug)]
pub struct FloatParameter<'session> {
    pub(crate) wrap: ParmNodeWrap<'session>,
}

#[derive(Debug)]
pub struct IntParameter<'session> {
    pub(crate) wrap: ParmNodeWrap<'session>,
}

#[derive(Debug)]
pub struct StringParameter<'session> {
    pub(crate) wrap: ParmNodeWrap<'session>,
}

#[derive(Debug)]
pub enum Parameter<'session> {
    Float(FloatParameter<'session>),
    Int(IntParameter<'session>),
    String(StringParameter<'session>),
    Other(BaseParameter<'session>),
}

impl<'node> Parameter<'node> {
    pub(crate) fn new(node: &'node HoudiniNode, info: ParmInfo<'node>) -> Parameter<'node> {
        let base = ParmNodeWrap { info, node };
        match base.info.parm_type() {
            ParmType::Int
            | ParmType::Button
            | ParmType::Toggle
            | ParmType::Folder
            | ParmType::Folderlist => Parameter::Int(IntParameter { wrap: base }),
            ParmType::Float | ParmType::Color => Parameter::Float(FloatParameter { wrap: base }),
            ParmType::String
            | ParmType::Node
            | ParmType::PathFile
            | ParmType::PathFileDir
            | ParmType::PathFileGeo
            | ParmType::PathFileImage => Parameter::String(StringParameter { wrap: base }),
            _ => Parameter::Other(BaseParameter { wrap: base }),
        }
    }
    pub fn info(&self) -> &ParmInfo {
        &self.base().info
    }

    pub fn name(&self) -> Result<Cow<'_, str>> {
        match self {
            Parameter::Float(p) => p.name(),
            Parameter::Int(p) => p.name(),
            Parameter::String(p) => p.name(),
            Parameter::Other(p) => p.wrap.info.name().map(|s| Cow::Owned(s)),
        }
    }

    pub fn parent(&self) -> Result<Option<Parameter>> {
        match self.info().parent_id() {
            ParmHandle(-1) => Ok(None),
            handle => {
                let node = self.base().node;
                let info = handle.info(node)?;
                Ok(Some(Parameter::new(node, info)))
            }
        }
    }

    pub(crate) fn base(&self) -> &ParmNodeWrap {
        match self {
            Parameter::Float(p) => &p.wrap,
            Parameter::Int(p) => &p.wrap,
            Parameter::String(p) => &p.wrap,
            Parameter::Other(p) => &p.wrap,
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
        crate::ffi::get_parm_float_values(&self.wrap.node, start, count)
    }

    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        crate::ffi::set_parm_float_values(
            &self.wrap.node,
            self.wrap.info.float_values_index(),
            self.wrap.info.size(),
            val.as_ref(),
        )
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
        crate::ffi::get_parm_int_values(&self.wrap.node, start, count)
    }

    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        let start = self.wrap.info.int_values_index();
        let count = self.wrap.info.size();
        crate::ffi::set_parm_int_values(&self.wrap.node, start, count, val.as_ref())
    }
}

impl<'s> ParmBaseTrait<'s> for StringParameter<'s> {
    type ValueType = String;

    fn wrap(&self) -> &ParmNodeWrap<'s> {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<String>> {
        let start = self.wrap.info.string_values_index();
        let count = self.wrap.info.size();
        crate::ffi::get_parm_string_values(&self.wrap.node, start, count)
    }

    // TODO Maybe take it out of the trait? AsRef makes it an extra String copy. Consider ToOwned?
    // What a hell did I mean by that?
    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        let start = self.wrap.info.string_values_index();
        let count = self.wrap.info.size();
        let c_str: std::result::Result<Vec<CString>, _> = val
            .as_ref()
            .into_iter()
            .map(|s| CString::new(s.clone()))
            .collect();
        crate::ffi::set_parm_string_values(&self.wrap.node, &self.wrap.info.id(), &c_str?)
    }
}

impl std::fmt::Debug for ParmNodeWrap<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parameter[{name} of type {type:?}]",
               name=self.info.name().unwrap(),
                type=self.info.parm_type())
    }
}
