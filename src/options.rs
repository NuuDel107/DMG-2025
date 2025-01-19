use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Options {
    pub rom_path: String,
    pub save_folder: String,
    pub window_scale: u8,
    pub audio_sample_rate: u32,
    pub start_immediately: bool,
    pub log: bool,
    pub breakpoints: Vec<u16>,
}

impl Options {
    pub fn load() -> Self {
        let options = fs::read_to_string("options.json5").unwrap();
        json5::from_str(&options).expect("Invalid options string")
    }
}
