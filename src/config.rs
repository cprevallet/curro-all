// Module containing user preferences, static information and helper utilities.
use crate::Error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// Meta-program info. Displayed in about dialog.
pub const ICON_NAME: &str = "siliconsneaker2";
pub const APP_ID: &str = "com.github.cprevallet.siliconsneaker2";
pub const PROGRAM_NAME: &str = "Aggregate";
pub const COPYRIGHT: &str = "© 2026";
pub const AUTHOR: &str = "Craig S. Prevallet <penguintx@hotmail.com>";
pub const ARTIST1: &str = "Amos Kofi Commey";
pub const TESTER1: &str = "Gabriel A. Prevallet";
pub const TESTER2: &str = "Jacob Thymann Nielsen";
pub const TESTER3: &str = "Tony Castronovo";
pub const SETTINGSFILE: &str = "siliconsneaker2_settings.toml";

//Unit of measure system.
#[allow(dead_code)]
pub enum Units {
    Metric,
    US,
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: i32,
    pub height: i32,
    pub main_split: i32,
    pub left_frame_split: i32,
    pub right_frame_split: i32,
    pub units_index: u32, // toml won't serialize enums, we'll use the selected DropDown
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            width: 800,
            height: 600,
            main_split: 200,
            left_frame_split: 200,
            right_frame_split: 200,
            units_index: 0,
        }
    }
}

/// Saves the WindowConfig struct to a TOML file.
pub fn save_config(config: &WindowConfig, path: &Path) -> std::io::Result<()> {
    // Use toml::to_string() to serialize the struct into a TOML string
    let toml_string = toml::to_string(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    // Write the TOML string to the specified file
    fs::write(path, toml_string)?;
    Ok(())
}

/// Attempts to parse a TOML string into a WindowConfig struct.
fn deserialize_config(toml_string: &str) -> std::result::Result<WindowConfig, Box<dyn Error>> {
    // Use toml::from_str for deserialization
    match toml::from_str(toml_string) {
        Ok(config) => Ok(config),
        // Convert the toml::de::Error into a boxed trait object
        Err(e) => Err(Box::new(e)),
    }
}

/// Loads the WindowConfig struct from a TOML file, using the dedicated
/// deserialize_config function, or returns the default config on failure.
pub fn load_config(path: &Path) -> WindowConfig {
    if !path.exists() {
        return WindowConfig::default();
    }

    // Read file content
    match fs::read_to_string(path) {
        Ok(toml_string) => {
            // Call the dedicated deserialization function
            match deserialize_config(&toml_string) {
                Ok(config) => config,
                Err(_e) => WindowConfig::default(),
            }
        }
        Err(_e) => WindowConfig::default(),
    }
}
