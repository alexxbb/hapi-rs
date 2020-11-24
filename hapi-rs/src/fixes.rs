#![allow(non_upper_case_globals)]
use crate::auto::rusty::{ParmType, StatusVerbosity, State};
use crate::auto::bindings as ffi;
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

impl From<i32> for State {
    fn from(s: i32) -> Self {
        match s {
            0 => State::StateReady,
            1 => State::FatalErrors,
            2 => State::CookErrors,
            3 => State::StartingCook,
            4=> State::StateCooking,
            5 => State::StartingLoad,
            6=>State::StateLoading,
            7=>State::StateMax,
            _=> unimplemented!()
        }
    }
}
