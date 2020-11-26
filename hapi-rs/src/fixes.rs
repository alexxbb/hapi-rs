#![allow(non_upper_case_globals)]
use crate::auto::bindings as ffi;
use crate::auto::rusty::{ParmType, State, StatusVerbosity};
impl ParmType {
    pub const IntStart: ParmType = ParmType::Int;
    pub const IntEnd: ParmType = ParmType::Button;
    pub const FloatStart: ParmType = ParmType::Float;
    pub const FloatEnd: ParmType = ParmType::Color;
    pub const StringStart: ParmType = ParmType::String;
    pub const StringEnd: ParmType = ParmType::Node;
    pub const PathStart: ParmType = ParmType::PathFile;
    pub const PathEnd: ParmType = ParmType::PathFileImage;
    pub const NodeStart: ParmType = ParmType::Node;
    pub const NodeEnd: ParmType = ParmType::Node;
    pub const ContainerStart: ParmType = ParmType::Folderlist;
    pub const ContainerEnd: ParmType = ParmType::FolderlistRadio;
    pub const NonvalueStart: ParmType = ParmType::Folder;
    pub const NonvalueEnd: ParmType = ParmType::Separator;
}

impl StatusVerbosity {
    pub const VerbosityAll: Self = Self::Statusverbosity2;
    pub const VerbosityErrors: Self = Self::Statusverbosity0;
    pub const VerbosityWarnings: Self = Self::Statusverbosity1;
    pub const VerbosityMessages: Self = Self::Statusverbosity2;
}

impl State {
    pub const MaxReadyState: State = State::ReadyWithCookErrors;
}

impl From<i32> for State {
    fn from(s: i32) -> Self {
        match s {
            0 => State::Ready,
            1 => State::ReadyWithFatalErrors,
            2 => State::ReadyWithCookErrors,
            3 => State::StartingCook,
            4 => State::Cooking,
            5 => State::StartingLoad,
            6 => State::Loading,
            7 => State::Max,
            _ => unimplemented!(),
        }
    }
}
