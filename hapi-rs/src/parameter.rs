use std::ffi::CString;

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

    fn name(&self) -> Result<String> {
        self.wrap().info.name()
    }
    fn is_menu(&self) -> bool {
        !matches!(self.wrap().info.choice_list_type(), ChoiceListType::None)
    }
    #[doc(hidden)]
    fn wrap(&self) -> &ParmNodeWrap;
    fn menu_items(&self) -> Result<Option<Vec<ParmChoiceInfo<'_>>>> {
        if !self.is_menu() {
            return Ok(None);
        }
        let wrap = self.wrap();
        let parms = crate::ffi::get_parm_choice_list(
            wrap.node,
            &wrap.info.session,
            wrap.info.choice_index(),
            wrap.info.choice_count(),
        );
        let parms = parms.map(|v| {
            v.into_iter()
                .map(|p| ParmChoiceInfo {
                    inner: p,
                    session: &wrap.info.session,
                })
                .collect::<Vec<ParmChoiceInfo<'_>>>()
        })?;
        Ok(Some(parms))
    }
    fn expression(&self, index: i32) -> Result<String> {
        let wrap = self.wrap();
        let name = wrap.info.name_cstr()?;
        crate::ffi::get_parm_expression(wrap.node, &wrap.info.session, &name, index)
    }

    fn has_expression(&self, index: i32) -> Result<bool> {
        let wrap = self.wrap();
        let name = wrap.info.name_cstr()?;
        crate::ffi::parm_has_expression(wrap.node, &wrap.info.session, &name, index)
    }

    fn set_expression(&self, value: &str, index: i32) -> Result<()> {
        let wrap = self.wrap();
        let value = CString::new(value)?;
        crate::ffi::set_parm_expression(
            wrap.node,
            &wrap.info.session,
            wrap.info.id(),
            &value,
            index,
        )
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>>;
    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>;
}

#[derive(Debug, Clone, Copy)]
pub struct ParmHandle(pub HAPI_ParmId, pub(crate) ());

impl ParmHandle {
    pub fn from_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        let name = CString::new(name)?;
        let id = crate::ffi::get_parm_id_from_name(&name, node.handle, &node.session)?;
        Ok(ParmHandle(id, ()))
    }
    pub fn info(&self, node: &HoudiniNode) -> Result<ParmInfo> {
        let info = crate::ffi::get_parm_info(node.handle, &node.session, *self)?;
        Ok(ParmInfo {
            inner: info,
            session: node.session.clone(),
        })
    }
}

impl ParmInfo {
    pub fn from_parm_name(name: &str, node: &HoudiniNode) -> Result<Self> {
        let name = CString::new(name)?;
        let info = crate::ffi::get_parm_info_from_name(node.handle, &node.session, &name);
        info.map(|info| ParmInfo {
            inner: info,
            session: node.session.clone(),
        })
    }

    pub fn into_node_parm(self, node: NodeHandle) -> Parameter {
        Parameter::new(node, self)
    }
}

#[derive(Debug)]
pub struct ParmNodeWrap {
    pub(crate) info: ParmInfo,
    pub(crate) node: NodeHandle,
}

#[derive(Debug)]
pub struct BaseParameter {
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct FloatParameter {
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct IntParameter {
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub struct StringParameter {
    pub(crate) wrap: ParmNodeWrap,
}

#[derive(Debug)]
pub enum Parameter {
    Float(FloatParameter),
    Int(IntParameter),
    String(StringParameter),
    Button(IntParameter),
    Other(BaseParameter),
}

impl Parameter {
    pub(crate) fn new(node: NodeHandle, info: ParmInfo) -> Parameter {
        let base = ParmNodeWrap { info, node };
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

    pub fn name(&self) -> Result<String> {
        self.info().name()
    }

    pub fn parent(&self) -> Result<Option<ParmInfo>> {
        let wrap = self.base();
        match wrap.info.parent_id() {
            ParmHandle(-1, ()) => Ok(None),
            handle => {
                let session = wrap.info.session.clone();
                let info = crate::ffi::get_parm_info(wrap.node, &session, handle)?;
                Ok(Some(ParmInfo {
                    inner: info,
                    session,
                }))
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

impl ParmBaseTrait for FloatParameter {
    type ValueType = f32;

    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>> {
        crate::ffi::get_parm_float_values(
            self.wrap.node,
            &self.wrap.info.session,
            self.wrap.info.float_values_index(),
            self.wrap.info.size(),
        )
    }

    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        crate::ffi::set_parm_float_values(
            self.wrap.node,
            &self.wrap.info.session,
            self.wrap.info.float_values_index(),
            self.wrap.info.size(),
            val.as_ref(),
        )
    }
}

impl ParmBaseTrait for IntParameter {
    type ValueType = i32;

    fn wrap(&self) -> &ParmNodeWrap {
        &self.wrap
    }

    fn get_value(&self) -> Result<Vec<Self::ValueType>> {
        crate::ffi::get_parm_int_values(
            self.wrap.node,
            &self.wrap.info.session,
            self.wrap.info.int_values_index(),
            self.wrap.info.size(),
        )
    }

    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        crate::ffi::set_parm_int_values(
            self.wrap.node,
            &self.wrap.info.session,
            self.wrap.info.int_values_index(),
            self.wrap.info.size(),
            val.as_ref(),
        )
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
        Ok(crate::ffi::get_parm_string_values(
            self.wrap.node,
            &self.wrap.info.session,
            self.wrap.info.string_values_index(),
            self.wrap.info.size(),
        )?
        .into_iter()
        .collect::<Vec<_>>())
    }

    // TODO Maybe take it out of the trait? AsRef makes it an extra String copy. Consider ToOwned?
    // What a hell did I mean by that?
    // Update: 2 month later still can't remember
    // Update: 3 month later. Still no clue, moving on for now
    fn set_value<T>(&self, val: T) -> Result<()>
    where
        T: AsRef<[Self::ValueType]>,
    {
        let c_str: std::result::Result<Vec<CString>, _> = val
            .as_ref()
            .iter()
            .map(|s| CString::new(s.clone()))
            .collect();
        crate::ffi::set_parm_string_values(
            self.wrap.node,
            &self.wrap.info.session,
            &self.wrap.info.id(),
            &c_str?,
        )
    }
}
