use super::*;

fn on_error(e: &std::io::Error) {
    eprintln!("Failed to save CPU state: {e}");
}

impl Window {
    fn get_save_path(&self) -> String {
        let path_list = self.options.rom_path.split("/").collect::<Vec<&str>>();
        let rom_name = path_list.last().unwrap();
        format!("{}{}.json5", self.options.save_folder, rom_name)
    }

    /// Saves current emulator state to file, a.k.a. serializes CPU
    pub fn save_state(&self) {
        println!("Saving CPU state to {}", self.get_save_path());
        let cpu = self.cpu.lock().unwrap();
        let state = json5::to_string::<CPU>(&cpu).expect("Couldn't serialize CPU state");
        
        let _ = fs::remove_file(self.get_save_path());
        let file_res = File::create(self.get_save_path()).inspect_err(on_error);

        if let Ok(mut file) = file_res {
            let _ = file.write_all(state.as_bytes()).inspect_err(on_error);
        } else {
            on_error(&file_res.unwrap_err());
        }
    }

    /// Loads saved emulator state from file, a.k.a. deserializes CPU 
    pub fn load_state(&mut self) {
        println!("Loading CPU state from {}", self.get_save_path());
        let save_file = fs::read_to_string(self.get_save_path()).inspect_err(on_error);
        if let Ok(save) = save_file {
            let mut cpu = self.cpu.lock().unwrap();
            let rom = cpu.mem.mbc.rom.clone();
            
            let mut loaded_cpu = json5::from_str::<CPU>(&save).expect("Couldn't deserialize CPU state");
            loaded_cpu.mem.mbc.load_rom(rom);
            *cpu = loaded_cpu;

        } else {
            on_error(&save_file.unwrap_err());
        }
    }
}