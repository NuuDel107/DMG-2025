use super::*;
use std::path::PathBuf;

fn on_error(e: &std::io::Error) {
    eprintln!("Failed to save CPU state: {e}");
}

impl Window {
    fn get_save_path(&self) -> PathBuf {
        let path_list = self.options.rom_path.split("/").collect::<Vec<&str>>();
        let rom_name = path_list.last().unwrap();
        PathBuf::new()
            .join(&self.options.data_path)
            .join("saves")
            .join(rom_name)
            .join(".json5")
    }

    /// Saves current emulator state to file, a.k.a. serializes CPU
    pub fn save_state(&self) {
        let cpu_option = self.cpu.lock().unwrap();
        if cpu_option.is_none() {
            return;
        }
        let path = self.get_save_path();
        println!("Saving CPU state to {}", path.to_str().unwrap());
        let state = json5::to_string::<CPU>(cpu_option.as_ref().unwrap())
            .expect("Couldn't serialize CPU state");

        let _ = fs::remove_file(&path);
        let file_res = File::create(&path).inspect_err(on_error);

        if let Ok(mut file) = file_res {
            let _ = file.write_all(state.as_bytes()).inspect_err(on_error);
        } else {
            on_error(&file_res.unwrap_err());
        }
    }

    /// Loads saved emulator state from file, a.k.a. deserializes CPU
    pub fn load_state(&mut self) {
        let mut cpu_option = self.cpu.lock().unwrap();
        if cpu_option.is_none() {
            return;
        }
        let path = self.get_save_path();
        println!("Loading CPU state from {}", path.to_str().unwrap());
        let save_file = fs::read_to_string(&path).inspect_err(on_error);

        if let Ok(save) = save_file {
            let rom = cpu_option.as_ref().unwrap().mem.mbc.rom.clone();
            let mut loaded_cpu =
                json5::from_str::<CPU>(&save).expect("Couldn't deserialize CPU state");
            loaded_cpu.mem.mbc.load_rom(rom);
            *cpu_option = Some(loaded_cpu);
        } else {
            on_error(&save_file.unwrap_err());
        }
    }
}
