use super::*;

impl Window {
    pub fn handle_input(&mut self, input: &egui::InputState, in_main_window: bool) {
        for event in &input.events {
            if let egui::Event::Key {
                key,
                pressed,
                repeat,
                ..
            } = event
            {
                use egui::Key;

                if *repeat && !matches!(*key, Key::F3 | Key::F4) {
                    continue;
                }

                if in_main_window {
                    if !self.rom_loaded || self.paused.load(Ordering::Relaxed) {
                        if let Some(rebind) = self.rebinding_input {
                            self.rebinding_input = None;
                            self.options.keybinds.insert(rebind, key.name().to_string());
                            self.options.save();
                            return;
                        }
                    }
                    let key_string = key.name().to_string();
                    for (input, key) in &self.options.keybinds {
                        if &key_string == key {
                            self.input_state.lock().unwrap().set(*input, !pressed);
                        }
                    }
                }

                if *pressed {
                    match *key {
                        // Toggle the clock
                        Key::Escape => {
                            if self.rom_loaded && self.paused.fetch_not(Ordering::Relaxed) {
                                self.rebinding_input = None;
                                self.start_clock();
                            };
                        }
                        // Toggle debug window
                        Key::F1 => self.show_debug = !self.show_debug,
                        // Toggle profiler window
                        Key::F2 => {
                            self.show_profiler = !self.show_profiler;
                            puffin::set_scopes_on(self.show_profiler);
                        }
                        // Manually step over an instruction
                        Key::F3 => {
                            if self.paused.load(Ordering::Relaxed) {
                                let _ = self
                                    .clock_tx
                                    .clone()
                                    .unwrap()
                                    .send(ExecutorInstruction::RunInstruction);
                            }
                        }
                        // Run until next frame
                        Key::F4 => {
                            if self.paused.load(Ordering::Relaxed) {
                                let _ = self
                                    .clock_tx
                                    .clone()
                                    .unwrap()
                                    .send(ExecutorInstruction::RunFrame);
                            }
                        }
                        // Run CPU profiling for one frame
                        Key::F5 => {
                            let mut cpu_option = self.cpu.lock().unwrap();
                            if cpu_option.is_some() {
                                cpu_option.as_mut().unwrap().profiling = true;
                            }
                        }
                        Key::F7 => {
                            self.save_state();
                        }
                        Key::F8 => {
                            self.load_state();
                        }
                        _ => {}
                    };
                }
            }
        }
    }
}
