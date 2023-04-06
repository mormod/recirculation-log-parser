use hdf5::{H5Type, types::VarLenUnicode};

#[derive(Debug, H5Type)]
#[repr(C)]
pub struct CanCmt {
    pub id: u32,
    pub ts: u32,
    pub content: VarLenUnicode,
}