use hdf5::H5Type;
use std::fmt;

#[derive(H5Type, Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(C)]
pub struct CanMsg {
    pub hex_id: u32,
    pub value: f32,
    pub ts: u32,
}

impl Eq for CanMsg {}

impl Ord for CanMsg {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hex_id.cmp(&other.hex_id)
    }
}

impl fmt::Display for CanMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {} = {}", self.hex_id, self.ts, self.value)
    }
}

impl Default for CanMsg {
    fn default() -> Self {
        Self {
            hex_id: 0,
            value: 0.0,
            ts: 0,
        }
    }
}
