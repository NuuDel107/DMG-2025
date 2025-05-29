use egui::{Color32, Key};
use serde::{de::Visitor, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io::Write};

use crate::cpu::input::InputFlag;

#[derive(Debug, Clone, PartialEq)]
/// Represents color palette for display
pub struct Palette(pub Color32, pub Color32, pub Color32, pub Color32);

impl Palette {
    pub fn original() -> Self {
        Self(
            Color32::WHITE,
            Color32::GRAY,
            Color32::DARK_GRAY,
            Color32::BLACK,
        )
    }

    pub fn lcd() -> Self {
        Self(
            Color32::from_rgb(224, 248, 208),
            Color32::from_rgb(136, 192, 112),
            Color32::from_rgb(52, 104, 86),
            Color32::from_rgb(8, 24, 32),
        )
    }

    fn color_from_hex(hex: String) -> Color32 {
        Color32::from_hex(&hex).expect("Hex string not valid!")
    }

    pub fn from_hex(array: [String; 4]) -> Self {
        Self(
            Self::color_from_hex(array[0].clone()),
            Self::color_from_hex(array[1].clone()),
            Self::color_from_hex(array[2].clone()),
            Self::color_from_hex(array[3].clone()),
        )
    }

    pub fn get_col(&self, index: u8) -> Color32 {
        match index {
            0 => self.0,
            1 => self.1,
            2 => self.2,
            3 => self.3,
            _ => panic!("Index too high: Only 4 colors in palette"),
        }
    }

    pub fn get_mut(&mut self, index: u8) -> &mut Color32 {
        match index {
            0 => &mut self.0,
            1 => &mut self.1,
            2 => &mut self.2,
            3 => &mut self.3,
            _ => panic!("Index too high: Only 4 colors in palette"),
        }
    }
}

impl Serialize for Palette {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(4))?;
        for i in 0..4 {
            let _ = seq.serialize_element(&self.get_col(i).to_hex());
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Palette {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(PaletteVisitor)
    }
}

struct PaletteVisitor;
impl<'de> Visitor<'de> for PaletteVisitor {
    type Value = Palette;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an array of 4 hex strings")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut array: [String; 4] = [String::new(), String::new(), String::new(), String::new()];
        for hex in &mut array {
            if let Ok(Some(hex_string)) = seq.next_element::<String>() {
                *hex = hex_string.to_string();
            } else {
                return Err(serde::de::Error::custom("Palette hex array not valid!"));
            }
        }
        Ok(Palette::from_hex(array))
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Options {
    pub data_path: String,
    pub rom_path: String,
    pub keybinds: HashMap<InputFlag, String>,
    pub window_scale: u8,
    pub palette_preset: u8,
    pub custom_palette: Palette,
    pub audio_sample_rate: u32,
    pub volume: u8,
}

impl Options {
    pub fn load() -> Self {
        let options_path = dirs_next::data_dir()
            .unwrap()
            .join("DMG-2025")
            .join("options.json");
        if !options_path.exists() {
            return Self::init_default();
        }
        let file = fs::read_to_string(options_path).unwrap();
        let json = serde_json::from_str(&file);
        if let Ok(options) = json {
            options
        } else {
            eprintln!("Options file outdated or corrupted. Restoring defaults");
            Self::init_default()
        }
    }

    fn init_default() -> Self {
        let options = Options::default();
        options.save();
        options
    }

    pub fn save(&self) {
        let default_folder = dirs_next::data_dir().unwrap().join("DMG-2025");
        if !default_folder.exists() {
            Self::init_folder(&default_folder);
        }

        let options_path = default_folder.join("options.json");
        let _ = fs::remove_file(&options_path);

        let mut file = fs::File::create(&options_path).unwrap();
        let json = serde_json::to_string_pretty(&self).unwrap();
        let _ = file.write_all(json.as_bytes());
    }

    fn init_folder(folder: &PathBuf) {
        let _ = fs::create_dir(folder);
        let _ = fs::create_dir(folder.join("saves"));
    }

    pub fn default_keybinds() -> HashMap<InputFlag, String> {
        HashMap::from([
            (InputFlag::RIGHT, Key::ArrowRight.name().to_string()),
            (InputFlag::LEFT, Key::ArrowLeft.name().to_string()),
            (InputFlag::UP, Key::ArrowUp.name().to_string()),
            (InputFlag::DOWN, Key::ArrowDown.name().to_string()),
            (InputFlag::A, Key::X.name().to_string()),
            (InputFlag::B, Key::Z.name().to_string()),
            (InputFlag::SELECT, Key::Backspace.name().to_string()),
            (InputFlag::START, Key::Enter.name().to_string()),
        ])
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
            keybinds: Self::default_keybinds(),
            window_scale: 4,
            palette_preset: 0,
            custom_palette: Palette::original(),
            audio_sample_rate: 48000,
            volume: 100,
        }
    }
}
