use std::borrow::Cow;
use std::ffi::CString;
use std::fmt::Formatter;

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

pub trait ParmBaseTrait {
    type ValueType;

    fn c_name(&self) -> Result<Cow<'_, CString>> {
        let info = &self.wrap().info;
        match info.name.as_ref() {
            None => Ok(Cow::Owned(info.name_cstr()?)),
            Some(n) => Ok(Cow::Borrowed(n)),
        }
    }

    fn name(&self) -> Result<Cow<'_, str>> {
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
    fn wrap(&self) -> &ParmNodeWrap;
    // TODO find a way to make it private
    fn info(&self) -> &ParmInfo {
        &self.wrap().info
    }
    fn menu_items(&self) -> Option<Result<Vec<ParmChoiceInfo<'_>>>> {
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
    fn expression(&self, index: i32) -> Result<String> {
        let wrap = self.wrap();
        crate::ffi::get_parm_expression(&wrap.node, &self.c_name()?, index)
    }

    fn has_expression(&self, index: i32) -> Result<bool> {
        crate::ffi::parm_has_expression(&self.wrap().node, &self.c_name()?, index)
    }

    fn set_expression(&self, value: &str, index: i32) -> Result<()> {
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
pub struct ParmHandle(pub HAPI_ParmId, pub(crate) ());

impl ParmHandle {
    pub fn from_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        let name = CString::new(name)?;
        let id = crate::ffi::get_parm_id_from_name(&name, node)?;
        Ok(ParmHandle(id, ()))
    }
    pub fn info(&self, node: &HoudiniNode) -> Result<ParmInfo> {
        let info = crate::ffi::get_parm_info(node, &self)?;
        Ok(ParmInfo {
            inner: info,
            session: node.session.clone(),
            name: None,
        })
    }
}

impl ParmInfo {
    pub fn from_parm_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        let name = CString::new(name)?;
        let info = crate::ffi::get_parm_info_from_name(&node, &name);
        info.map(|info| ParmInfo {
            inner: info,
            session: node.session.clone(),
            name: Some(name),
        })
    }
}

// TODO: Should be private
pub struct ParmNodeWrap {
    pub(crate) info: ParmInfo,
    pub(crate) node: HoudiniNode,
}

#[derive(Debug)]
pub struct BaseParameter {
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct FloatParameter{
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct IntParameter{
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct StringParameter{
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub enum Parameter{
    Float(FloatParameter),
    Int(IntParameter),
    String(StringParameter),
    Button(IntParameter),
    Other(BaseParameter),
}

impl Parameter{
    pub(crate) fn new(node: &HoudiniNode, info: ParmInfo) -> Parameter {
        let base = ParmNodeWrap { info, node: node.clone() };
        match base.info.parm_type() {
            ParmType::Int | ParmType::Toggle | ParmType::Folder | ParmType::Folderlist => {
                Parameter::Int(IntParameter { wrap: base })
            }
            ParmType::Button => Parameter::Button(IntParameter { wrap: base }),
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
            Parameter::Button(p) => p.name(),
            Parameter::String(p) => p.name(),
            Parameter::Other(p) => p.wrap.info.name().map(Cow::Owned),
        }
    }

    pub fn parent(&self) -> Result<Option<ParmInfo>> {
        match self.info().parent_id() {
            ParmHandle(-1, ()) => Ok(None),
            handle => {
                Ok(Some(handle.info(&self.base().node)?))
            }
        }
    }

    pub(crate) fn base(&self) -> &ParmNodeWrap {
        match self {
            Parameter::Float(p) => &p.wrap,
            Parameter::Int(p) => &p.wrap,
            Parameter::Button(p) => &p.wrap,
            Parameter::String(p) => &p.wrap,
            Parameter::Other(p) => &p.wrap,
        }
    }
}

impl ParmBaseTrait for FloatParameter{
    type ValueType = f32;

    fn wrap(&self) -> &ParmNodeWrap{
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

impl ParmBaseTrait for IntParameter{
    type ValueType = i32;

    fn wrap(&self) -> &ParmNodeWrap{
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

impl IntParameter {
    pub fn press_button(&self) -> Result<()> {
        if !matches!(self.wrap.info.parm_type(), ParmType::Button) {
            warn!("Parm {} not a Button type", self.wrap.info.name()?);
        }
        self.set_value([1])
    }
}

impl ParmBaseTrait for StringParameter {
    type ValueType = String;

    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<String>> {
        let start = self.wrap.info.string_values_index();
        let count = self.wrap.info.size();
        Ok(
            crate::ffi::get_parm_string_values(&self.wrap.node, start, count)?
                .into_iter()
                .collect::<Vec<_>>(),
        )
    }

    // TODO Maybe take it out of the trait? AsRef makes it an extra String copy. Consider ToOwned?
    // What a hell did I mean by that?
    // Update: 2 month later still can't remember
    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        // let start = self.wrap.info.string_values_index();
        // let count = self.wrap.info.size();
        let c_str: std::result::Result<Vec<CString>, _> = val
            .as_ref()
            .iter()
            .map(|s| CString::new(s.clone()))
            .collect();
        crate::ffi::set_parm_string_values(&self.wrap.node, &self.wrap.info.id(), &c_str?)
    }
}

impl std::fmt::Debug for ParmNodeWrap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parameter[{name} of type {type:?}]",
               name=self.info.name().unwrap(),
                type=self.info.parm_type())
    }
}
