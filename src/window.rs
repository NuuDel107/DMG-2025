use super::{
    cpu::{input::*, interrupts::*, registers::*},
    Options, CPU,
};
use egui::{epaint::*, TextureOptions};
use rodio::{
    buffer::SamplesBuffer,
    queue::{queue, SourcesQueueInput},
    OutputStream, Source,
};
use std::fs::{self, File, OpenOptions};
use std::io::{prelude::*, LineWriter};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::Duration;

mod debug;
mod instructions;
mod logging;
mod saving;
use instructions::InstructionDB;

pub struct Window {
    cpu: Arc<Mutex<CPU>>,
    ctx: Arc<Mutex<Option<egui::Context>>>,
    cpu_running: Arc<AtomicBool>,
    clock_tx: Option<mpsc::SyncSender<bool>>,

    display_texture: Arc<Mutex<TextureHandle>>,
    _stream: OutputStream,
    audio_queue: Arc<SourcesQueueInput<f32>>,
    input_state: Arc<Mutex<InputFlag>>,

    options: Options,
    instruction_db: InstructionDB,

    has_initialized: bool,
    show_debug: bool,
    show_instruction_info: bool,
}

impl Window {
    pub fn new(cpu: CPU, options: Options, cc: &eframe::CreationContext<'_>) -> Window {
        // Initialize display texture with just white
        let texture = cc.egui_ctx.load_texture(
            "display",
            ColorImage::from_gray([160, 144], &[255; 160 * 144]),
            TextureOptions::NEAREST,
        );
        // Initialize audio queue and playback
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let (queue, queue_output) = queue(true);
        let _ = stream_handle
            .play_raw(queue_output.convert_samples())
            .inspect_err(|e| eprintln!("Failed to start queue playback: {e}"));

        Window {
            cpu: Arc::new(Mutex::new(cpu)),
            ctx: Arc::new(Mutex::new(None)),
            cpu_running: Arc::new(AtomicBool::new(options.start_immediately)),
            clock_tx: None,

            display_texture: Arc::new(Mutex::new(texture)),
            _stream: stream,
            audio_queue: queue,
            input_state: Arc::new(Mutex::new(InputFlag::from_bits_truncate(0xFF))),

            options,
            instruction_db: InstructionDB::init(),

            has_initialized: false,
            show_debug: false,
            show_instruction_info: false,
        }
    }

    fn init(&mut self) {
        // Clear logfile
        if self.options.log {
            let _ = fs::remove_file("log.txt");
            let _ = File::create("log.txt");
        }
        // Start loop on thread that receives clock signal
        // and cycles the CPU
        // Function returns the sender channel
        self.clock_tx = Some(self.start_executor());

        // Start clock if CPU should start immediately
        if self.options.start_immediately {
            self.start_clock();
        }
    }

    fn start_executor(&mut self) -> mpsc::SyncSender<bool> {
        let cpu_ref = Arc::clone(&self.cpu);
        let ctx_ref = Arc::clone(&self.ctx);
        let display_ref = Arc::clone(&self.display_texture);
        let running_ref = Arc::clone(&self.cpu_running);
        let input_ref = Arc::clone(&self.input_state);
        let audio_queue_ref = Arc::clone(&self.audio_queue);

        let options = self.options.clone();
        let mut logfile = if options.log {
            Some(LineWriter::new(
                OpenOptions::new().write(true).open("log.txt").unwrap(),
            ))
        } else {
            None
        };

        let (tx, rx) = mpsc::sync_channel::<bool>(0);
        thread::spawn(move || {
            loop {
                // Wait for timer
                let run_until_vblank = rx.recv().unwrap();
                let mut cpu = cpu_ref.lock().unwrap();

                // Update input
                let input = input_ref.lock().unwrap();
                cpu.update_input(&input);
                drop(input);

                // If message was sent from main clock, run emulation until next VBlank
                if run_until_vblank {
                    loop {
                        // If program counter is at specified breakpoint,
                        // stop the clock
                        if options.breakpoints.contains(&cpu.reg.pc) {
                            cpu.breakpoint();
                            running_ref.store(false, Ordering::Relaxed);
                            break;
                        }

                        if logfile.is_some() {
                            #[allow(clippy::unnecessary_unwrap)]
                            Self::log(logfile.as_mut().unwrap(), &cpu);
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
                                    1,
                                    options.audio_sample_rate,
                                    cpu.apu.receive_buffer(),
                                )
                                .convert_samples(),
                            );
                            // Drop CPU before requesting repaint
                            drop(cpu);
                            // Request repaint to refresh display
                            egui::Context::request_repaint(
                                ctx_ref.lock().unwrap().as_ref().unwrap(),
                            );
                            break;
                        }
                    }
                }
                // Otherwise only execute one instruction manually
                else {
                    cpu.execute();
                    // Clear APU buffer
                    cpu.apu.receive_buffer();
                    egui::Context::request_repaint(ctx_ref.lock().unwrap().as_ref().unwrap());
                }
            }
        });
        tx
    }

    fn start_clock(&mut self) {
        let tx = self.clock_tx.as_ref().unwrap().clone();
        let running_ref = Arc::clone(&self.cpu_running);

        thread::spawn(move || loop {
            // Stop the loop when clock gets paused
            if !running_ref.load(Ordering::Relaxed) {
                break;
            }
            let _ = tx.send(true);
            // Wait for the duration between VBlanks (59.7 hZ)
            thread::sleep(Duration::from_micros(16742));
        });
    }

    fn handle_input(&mut self, input: &egui::InputState, in_main_window: bool) {
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
                    let input_option = match *key {
                        Key::ArrowRight => Some(InputFlag::RIGHT),
                        Key::ArrowLeft => Some(InputFlag::LEFT),
                        Key::ArrowUp => Some(InputFlag::UP),
                        Key::ArrowDown => Some(InputFlag::DOWN),
                        Key::X => Some(InputFlag::A),
                        Key::Z => Some(InputFlag::B),
                        Key::Backspace => Some(InputFlag::SELECT),
                        Key::Enter => Some(InputFlag::START),
                        _ => None,
                    };
                    if let Some(input) = input_option {
                        self.input_state.lock().unwrap().set(input, !pressed);
                    }
                }

                if *pressed {
                    match *key {
                        // Toggle the clock
                        Key::Space => {
                            if !self.cpu_running.fetch_not(Ordering::Relaxed) {
                                self.start_clock();
                            };
                        }
                        // Toggle debug window
                        Key::F1 => self.show_debug = !self.show_debug,
                        // Manually step over an instruction
                        Key::F3 => {
                            if !self.cpu_running.load(Ordering::Relaxed) {
                                let _ = self.clock_tx.clone().unwrap().send(false);
                            }
                        }
                        // Run until next frame
                        Key::F4 => {
                            if !self.cpu_running.load(Ordering::Relaxed) {
                                let _ = self.clock_tx.clone().unwrap().send(true);
                            }
                        }
                        // Manually activate breakpoint
                        Key::F5 => {
                            self.cpu.lock().unwrap().breakpoint();
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

impl eframe::App for Window {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.has_initialized {
            self.has_initialized = true;
            self.ctx = Arc::new(Mutex::new(Some(ctx.clone())));

            self.init();
        }

        // Render the main display window
        let central_frame = egui::Frame::central_panel(&ctx.style()).inner_margin(Margin::ZERO);
        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| {
                ctx.input(|input| {
                    self.handle_input(input, true);
                });

                // Calculate how texture should be resized on the screen to keep aspect ratio
                let screen_size = ui.available_size();
                let mut rect = ui.allocate_space(screen_size).1;
                let x_diff = rect.width() / 160.0;
                let y_diff = rect.height() / 144.0;
                let offset: Vec2 = if x_diff > y_diff {
                    rect.set_width(rect.width() / (x_diff / y_diff));
                    Vec2::new((screen_size.x - rect.width()) / 2.0, 0.0)
                } else {
                    rect.set_height(rect.height() / (y_diff / x_diff));
                    Vec2::new(0.0, (screen_size.y - rect.height()) / 2.0)
                };
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));

                // Paint pixel texture
                ui.painter().image(
                    self.display_texture.lock().unwrap().id(),
                    rect.translate(offset),
                    uv,
                    Color32::WHITE,
                );
            });

        if self.show_debug {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("debug_window"),
                egui::ViewportBuilder::default()
                    .with_title("Debug Window")
                    .with_inner_size([600.0, 400.0]),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ctx.input(|input| {
                            self.handle_input(input, false);
                        });
                        // Run rendering code
                        self.render_debug(ctx, ui);
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent viewport that we should not show next frame:
                        self.show_debug = false;
                    }
                },
            );
        }
    }
}
