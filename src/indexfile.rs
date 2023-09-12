use std::{
    collections::HashMap,
    fs::{self, Metadata},
    io,
    path::Path,
    time::SystemTime,
};

use crate::repr_file::ReprFile;

#[derive(Debug, PartialEq, Eq)]
pub struct IndexFile {
    size: u64,
    last_modified: Option<u64>,
}

impl IndexFile {
    pub fn new_from_metadata(metadata: &Metadata) -> Self {
        Self {
            size: metadata.len(),
            last_modified: metadata
                .modified()
                .ok()
                .and_then(|v| v.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|v| v.as_secs()),
        }
    }
    pub fn from_path(path: &Path) -> io::Result<Result<Self, String>> {
        Ok(Self::load(&fs::read_to_string(path)?))
    }
}

impl ReprFile for IndexFile {
    fn save(&self) -> String {
        let mut o = format!("Len={}\n", self.size);
        if let Some(age) = self.last_modified {
            o.push_str(&format!("Age={}\n", age));
        }
        o
    }
    fn load(src: &str) -> Result<Self, String> {
        let hm = HashMap::load(src)?;
        if let Some(len) = hm.get("Len").and_then(|len_str| len_str.parse().ok()) {
            let age = hm.get("Age").and_then(|lm_str| lm_str.parse().ok());
            Ok(Self {
                size: len,
                last_modified: age,
            })
        } else {
            return Err(format!("no Len in IndexFile!"));
        }
    }
}
