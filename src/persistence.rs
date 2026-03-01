use serde::{de::DeserializeOwned, Serialize};
use std::{fs, path::Path};

pub fn load_ron<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    ron::from_str(&contents).map_err(|e| format!("Failed to parse {}: {e}", path.display()))
}

pub fn save_ron<T: Serialize>(data: &T, path: &Path) -> Result<(), String> {
    let pretty = ron::ser::PrettyConfig::default();
    let contents = ron::ser::to_string_pretty(data, pretty)
        .map_err(|e| format!("Failed to serialize: {e}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
    }
    fs::write(path, contents)
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

pub fn load_ron_or_default<T: DeserializeOwned + Default>(path: &Path) -> T {
    match fs::read_to_string(path) {
        Ok(text) => ron::from_str(&text).unwrap_or_default(),
        Err(_) => T::default(),
    }
}
