use std::fmt::{self, Debug};

#[derive(Debug, Clone)]
pub struct CanId {
    pub hex_id: u32,
    pub str_id: Option<String>,
    pub description: Option<String>,
    pub scale: Option<f32>,
    pub unit: Option<String>,
}

impl CanId {
    pub fn empty_with_id(hex_id: u32) -> Self {
        let mut new = Self::default();
        new.hex_id = hex_id;
        new
    }
}

impl Default for CanId {
    fn default() -> Self {
        CanId {
            hex_id: 0,
            str_id: None,
            description: None,
            scale: None,
            unit: None
        }
    }
}

impl PartialEq for CanId {
    fn eq(&self, other: &Self) -> bool {
        self.hex_id == other.hex_id
    }
}

impl Eq for CanId {}


impl fmt::Display for CanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scale = match self.scale {
            Some(s) => s.to_string(),
            None => "None".to_string(),
        };
        write!(f, "{} = {} ({}) [{}, {}]", 
            self.str_id.clone().unwrap_or("None".to_string()),
            self.hex_id, 
            self.description.clone().unwrap_or("None".to_string()), 
            scale, 
            self.unit.clone().unwrap_or("None".to_string())
        )
    }
}
