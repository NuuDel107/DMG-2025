use super::*;

#[derive(PartialEq)]
pub enum ExecutorInstruction {
    RunFrame,
    RunInstruction,
    Stop,
}

impl Window {
    pub fn start_executor(&mut self) -> mpsc::SyncSender<ExecutorInstruction> {
        let cpu_ref = Arc::clone(&self.cpu);
        let ctx = self.ctx.clone();
        let display_ref = Arc::clone(&self.display_texture);
        let paused_ref = Arc::clone(&self.paused);
        let input_ref = Arc::clone(&self.input_state);
        let audio_queue_ref = Arc::clone(&self.audio_queue);

        let options = self.options.clone();

        let (tx, rx) = mpsc::sync_channel::<ExecutorInstruction>(0);
        thread::spawn(move || {
            loop {
                // Wait for timer
                let instruction = rx.recv().unwrap();
                if instruction == ExecutorInstruction::Stop {
                    break;
                }

                let mut cpu_option = cpu_ref.lock().unwrap();
                let cpu = cpu_option.as_mut().unwrap();

                // Update input
                let input = input_ref.lock().unwrap();
                cpu.update_input(&input);
                drop(input);

                // Run emulation until next VBlank
                if instruction == ExecutorInstruction::RunFrame {
                    loop {
                        // If program counter is at specified breakpoint,
                        // stop the clock
                        if options.breakpoints.contains(&cpu.reg.pc) {
                            cpu.breakpoint();
                            paused_ref.store(false, Ordering::Relaxed);
                            break;
                        }

                        // Break loop if execution function returns true (meaning VBlank was hit)
                        if cpu.execute() {
                            // Update display texture
                            let mut pixels = vec![];
                            for y in 0..144 {
                                for x in 0..160 {
                                    // Loop through front display
                                    let pixel = cpu.ppu.display[x][y];
                                    let color = match pixel {
                                        0 => Color32::WHITE,
                                        1 => Color32::GRAY,
                                        2 => Color32::DARK_GRAY,
                                        3 => Color32::BLACK,
                                        _ => unreachable!(),
                                    };
                                    pixels.push(color);
                                }
                            }
                            display_ref.lock().unwrap().set(
                                ColorImage {
                                    size: [160, 144],
                                    pixels,
                                },
                                TextureOptions::NEAREST,
                            );
                            // Append currently sampled audio buffer to playback queue
                            audio_queue_ref.append(
                                SamplesBuffer::new(
                                    2,
                                    options.audio_sample_rate,
                                    cpu.apu.receive_buffer(),
                                )
                                .convert_samples(),
                            );
                            // Request repaint to refresh display
                            egui::Context::request_repaint(&ctx);
                            break;
                        }
                    }
                }
                // Otherwise only execute one instruction manually
                else {
                    cpu.execute();
                    // Clear APU buffer
                    cpu.apu.receive_buffer();
                    egui::Context::request_repaint(&ctx);
                }
            }
        });
        tx
    }

    pub fn start_clock(&mut self) {
        let tx = self.clock_tx.as_ref().unwrap().clone();
        let paused_ref = Arc::clone(&self.paused);

        thread::spawn(move || loop {
            // Stop the loop when clock gets paused
            if paused_ref.load(Ordering::Relaxed) {
                break;
            }
            let _ = tx.send(ExecutorInstruction::RunFrame);
            // Wait for the duration between VBlanks (59.7 hZ)
            thread::sleep(Duration::from_micros(16742));
        });
    }
}
