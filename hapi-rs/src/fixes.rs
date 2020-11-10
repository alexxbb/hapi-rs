#![allow(non_upper_case_globals)]
use crate::auto::rusty::ParmType;
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
