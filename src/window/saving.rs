use super::*;
use std::path::PathBuf;

fn on_save_error(e: &std::io::Error) {
    eprintln!("Failed to save CPU state: {e}");
}

fn on_load_error(e: &std::io::Error) {
    eprintln!("Failed to load CPU state: {e}");
}

impl Window {
    fn get_save_path(&self) -> PathBuf {
        PathBuf::from(&self.options.data_path)
            .join("saves")
            .join(PathBuf::from(&self.options.rom_path).file_name().unwrap())
            .with_extension("json")
    }

    /// Saves current emulator state to file, a.k.a. serializes CPU
    pub fn save_state(&self) {
        if !self.rom_loaded {
            return;
        }
        let cpu_option = self.cpu.lock().unwrap();
        let path = self.get_save_path();
        println!("Saving CPU state to {}", path.to_str().unwrap());
        let state = serde_json::to_string::<CPU>(cpu_option.as_ref().unwrap())
            .expect("Failed to save CPU state: Serialization failed");

        let _ = fs::remove_file(&path);
        let file_res = File::create(&path).inspect_err(on_save_error);

        if let Ok(mut file) = file_res {
            let _ = file.write_all(state.as_bytes()).inspect_err(on_save_error);
        } else {
            on_save_error(&file_res.unwrap_err());
        }
    }

    /// Loads saved emulator state from file, a.k.a. deserializes CPU
    pub fn load_state(&mut self) {
        if !self.rom_loaded {
            return;
        }

        let path = self.get_save_path();
        println!("Loading CPU state from {}", path.to_str().unwrap());
        let save_file = fs::read_to_string(&path).inspect_err(on_load_error);

        if let Ok(save) = save_file {
            self.paused.store(true, Ordering::Relaxed);
            let mut cpu_option = self.cpu.lock().unwrap();

            // Initialize new CPU from deserialized state using current ROM file
            let rom = cpu_option.as_ref().unwrap().mem.mbc.rom.clone();
            let cpu_res = serde_json::from_str::<CPU>(&save);
            if let Ok(mut loaded_cpu) = cpu_res {
                loaded_cpu.mem.mbc.load_rom(rom);
                *cpu_option = Some(loaded_cpu);
            } else {
                eprintln!("Failed to load CPU state: Deserialization failed")
            }
            drop(cpu_option);

            // Start executing
            self.paused.store(false, Ordering::Relaxed);
            self.clock_tx = Some(self.start_executor());
            self.start_clock();
        }
    }
}
