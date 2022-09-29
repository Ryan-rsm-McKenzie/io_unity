use binrw::binrw;

use crate::{until::binrw_parser::U8Bool, SerializedFileMetadata};

use super::component::Component;

#[binrw]
#[brw(import_raw(args: SerializedFileMetadata))]
#[derive(Debug)]
pub struct Behaviour {
    #[brw(args_raw = args)]
    component: Component,
    #[brw(align_after(4))]
    enabled: U8Bool,
}