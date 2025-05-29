use super::*;

#[derive(PartialEq)]
pub enum ExecutorInstruction {
    RunFrame,
    RunInstruction,
    OptionsUpdated(Options),
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

        let mut options = self.options.clone();

        let (tx, rx) = mpsc::sync_channel::<ExecutorInstruction>(0);
        profiling::register_thread!("CPU");
        thread::spawn(move || {
            loop {
                // Wait for timer
                let received = rx.recv();
                // If message is error, clock has been stopped
                if received.is_err() {
                    break;
                }
                let instruction = received.unwrap();

                match instruction {
                    ExecutorInstruction::Stop => break,
                    ExecutorInstruction::OptionsUpdated(new_options) => {
                        options = new_options;
                        continue;
                    }
                    _ => {}
                };

                let mut cpu_option = cpu_ref.lock().unwrap();
                let cpu = cpu_option.as_mut().unwrap();

                // Update input
                let input = input_ref.lock().unwrap();
                cpu.update_input(&input);
                drop(input);

                // Run emulation until next VBlank
                if instruction == ExecutorInstruction::RunFrame {
                    profiling::scope!("CPU Frame");
                    loop {
                        // Break loop if execution function returns true (meaning VBlank was hit)
                        if cpu.execute() {
                            // If profiling was enabled, disable it after frame and pause profiler
                            // data so frame data can be viewed
                            if cpu.profiling {
                                cpu.profiling = false;
                                puffin::set_scopes_on(false);
                            }
                            // Update display texture
                            let image = Self::get_display_texture(cpu, &options);
                            display_ref
                                .lock()
                                .unwrap()
                                .set(image, TextureOptions::NEAREST);
                            // Append currently sampled audio buffer to playback queue
                            audio_queue_ref.append(
                                SamplesBuffer::new(
                                    2,
                                    options.audio_sample_rate,
                                    cpu.apu.receive_buffer(),
                                )
                                .amplify((options.volume as f32) / 100.0),
                            );
                            drop(cpu_option);
                            // Request repaint to refresh display
                            if !paused_ref.load(Ordering::Relaxed) {
                                ctx.request_repaint();
                            }
                            break;
                        }
                    }
                }
                // Otherwise only execute one instruction manually
                else {
                    cpu.execute();
                    // Clear APU buffer
                    cpu.apu.receive_buffer();
                    ctx.request_repaint();
                }
            }
        });
        tx
    }

    pub fn get_display_texture(cpu: &CPU, options: &Options) -> ColorImage {
        let palette = match options.palette_preset {
            0 => Palette::original(),
            1 => Palette::lcd(),
            2 => options.custom_palette.clone(),
            _ => unreachable!(),
        };
        let mut pixels = vec![];
        for y in 0..144 {
            for x in 0..160 {
                // Loop through front display
                let pixel = cpu.ppu.display[x][y];
                let color = palette.get_col(pixel);
                pixels.push(color);
            }
        }
        ColorImage {
            size: [160, 144],
            pixels,
        }
    }

    pub fn start_clock(&mut self) {
        let tx = self.clock_tx.as_ref().unwrap().clone();
        let paused_ref = Arc::clone(&self.paused);

        thread::spawn(move || loop {
            // Stop the loop when clock gets paused
            if paused_ref.load(Ordering::Relaxed) {
                break;
            }
            let res = tx.send(ExecutorInstruction::RunFrame);
            // If send returns error, CPU has probably been reloaded
            if res.is_err() {
                break;
            }
            // Wait for the duration between VBlanks (59.7 hZ)
            thread::sleep(Duration::from_micros(16742));
        });
    }
}
