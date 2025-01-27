use super::*;
use std::path::PathBuf;
use std::fs::OpenOptions;
use memmap2::MmapMut;

fn on_save_error(e: &std::io::Error) {
    eprintln!("Failed to save CPU state: {e}");
}

fn on_load_error(e: &std::io::Error) {
    eprintln!("Failed to load CPU state: {e}");
}

impl Window {
    pub fn get_save_folder(&self) -> PathBuf {
        PathBuf::from(&self.options.data_path)
            .join("saves")
            .join(PathBuf::from(&self.options.rom_path).file_stem().unwrap())
    }

    pub fn get_state_path(&self) -> PathBuf {
        self.get_save_folder().join(format!("state{}.json", self.state_slot))
    }

    fn get_mmap(&self) -> (MmapMut, bool) {
        let file_path = self.get_save_folder().join("save.bin");
        let created_new = !file_path.exists();
        let file = if created_new {
            OpenOptions::new().read(true).write(true).create_new(true).open(file_path).expect("Couldn't create new save file")
        } else {
            OpenOptions::new().read(true).write(true).open(file_path).expect("Couldn't open save file")
        };
        (unsafe { MmapMut::map_mut(&file).expect("Couldn't initialize memory map") }, created_new)
    }

    pub fn load_ram(&mut self) {
        let mut cpu_option = self.cpu.lock().unwrap();
        let cpu = cpu_option.as_mut().unwrap();
        // RAM is only saved in cartridges with battery
        if !cpu.mem.info.has_battery {
            return;
        }
        let (mmap, created_new) = self.get_mmap();
        cpu.mem.mbc.load_memory_map(mmap, created_new);
    }

    /// Saves current emulator state to file, a.k.a. serializes CPU
    pub fn save_state(&self) {
        if !self.rom_loaded {
            return;
        }
        let cpu_option = self.cpu.lock().unwrap();
        let path = self.get_state_path();
        println!("Saving CPU state to {}", path.to_str().unwrap());
        let state = serde_json::to_string::<CPU>(cpu_option.as_ref().unwrap())
            .expect("Failed to save CPU state: Serialization failed");

        if !self.get_save_folder().exists() {
            let _ = fs::create_dir(self.get_save_folder());
        }
        let _ = fs::remove_file(&path);
        let file_res = File::create(&path).inspect_err(on_save_error);

        if let Ok(mut file) = file_res {
            let _ = file.write_all(state.as_bytes()).inspect_err(on_save_error);
        }
    }

    /// Loads saved emulator state from file, a.k.a. deserializes CPU
    pub fn load_state(&mut self) {
        if !self.rom_loaded {
            return;
        }

        let path = self.get_state_path();
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
                if loaded_cpu.mem.info.has_battery {
                    loaded_cpu.mem.mbc.load_memory_map(self.get_mmap().0, true);
                }
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
