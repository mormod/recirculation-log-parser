use hdf5::{types::VarLenUnicode, H5Type};

#[derive(Debug, H5Type)]
#[repr(C)]
pub struct CanCmt {
    pub id: u32,
    pub ts: u64,
    pub value: VarLenUnicode,
}
