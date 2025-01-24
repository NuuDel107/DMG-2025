use serde::{Deserialize, Serialize};
use std::{fs, io::Write};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Options {
    pub data_path: String,
    pub rom_path: String,
    pub window_scale: u8,
    pub audio_sample_rate: u32,
    pub log: bool,
    pub breakpoints: Vec<u16>,
}

impl Options {
    pub fn load() -> Self {
        let options_path = dirs_next::data_dir()
            .unwrap()
            .join("DMG-2025")
            .join("options.json5");
        if !options_path.exists() {
            let options = Options::default();
            options.save();
            return options;
        }

        let options = fs::read_to_string(options_path).unwrap();
        json5::from_str(&options).expect("Invalid options string")
    }

    pub fn save(&self) {
        let default_folder = dirs_next::data_dir().unwrap().join("DMG-2025");
        if !default_folder.exists() {
            let _ = fs::create_dir(&default_folder);
        }

        let options_path = default_folder.join("options.json5");
        if options_path.exists() {
            let _ = fs::remove_file(&options_path);
        }

        let mut file = fs::File::create(&options_path).unwrap();
        let json = json5::to_string(&self).unwrap();
        let _ = file.write_all(json.as_bytes());
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            data_path: dirs_next::data_dir()
                .unwrap()
                .join("DMG-2025")
                .to_str()
                .unwrap()
                .into(),
            rom_path: String::new(),
            window_scale: 4,
            audio_sample_rate: 48000,
            log: false,
            breakpoints: vec![],
        }
    }
}
