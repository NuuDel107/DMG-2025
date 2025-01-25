use super::cpu::{input::*, interrupts::*, registers::*};
use super::*;
use egui::{epaint::*, FontData, FontDefinitions, TextureOptions};
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

mod clock;
use clock::ExecutorInstruction;
mod debug;
mod menu;
mod saving;

#[derive(PartialEq)]
pub enum MenuPage {
    Main,
    Options,
    Info,
}

pub struct Window {
    cpu: Arc<Mutex<Option<CPU>>>,
    ctx: Arc<egui::Context>,
    paused: Arc<AtomicBool>,
    rom_loaded: bool,
    clock_tx: Option<mpsc::SyncSender<ExecutorInstruction>>,

    display_texture: Arc<Mutex<TextureHandle>>,
    _stream: OutputStream,
    audio_queue: Arc<SourcesQueueInput<f32>>,
    input_state: Arc<Mutex<InputFlag>>,

    options: Options,

    menu_page: MenuPage,
    logo_texture: TextureHandle,
    arrow_texture: TextureHandle,
    show_debug: bool,
    show_color_picker: bool,
}

impl Window {
    /// Loads texture from included bytes
    /// Side note: why does this need to be so fucking hard
    fn load_texture(cc: &eframe::CreationContext<'_>, name: &str, data: &[u8]) -> TextureHandle {
        let img_data = ::image::load_from_memory(data).unwrap();
        let img = ColorImage::from_rgba_unmultiplied(
            [img_data.width() as usize, img_data.height() as usize],
            img_data.to_rgba8().as_flat_samples().as_slice(),
        );
        cc.egui_ctx.load_texture(name, img, TextureOptions::NEAREST)
    }

    pub fn new(options: Options, cc: &eframe::CreationContext<'_>) -> Window {
        // Initialize display texture with just white
        let display_texture = cc.egui_ctx.load_texture(
            "display",
            ColorImage::from_gray([160, 144], &[255; 160 * 144]),
            TextureOptions::NEAREST,
        );
        // Load UI textures
        let logo_texture = Self::load_texture(cc, "logo", include_bytes!("../assets/logo.png"));
        let arrow_texture = Self::load_texture(cc, "arrow", include_bytes!("../assets/arrow.png"));

        // Initialize audio queue and playback
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let (queue, queue_output) = queue(true);
        let _ = stream_handle
            .play_raw(queue_output.convert_samples())
            .inspect_err(|e| eprintln!("Failed to start queue playback: {e}"));
        // Load UI fonts
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "pixelmix".to_owned(),
            std::sync::Arc::new(FontData::from_static(include_bytes!(
                "../assets/pixelmix.ttf"
            ))),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "pixelmix".to_owned());
        fonts.font_data.insert(
            "pixelmix_bold".to_owned(),
            std::sync::Arc::new(FontData::from_static(include_bytes!(
                "../assets/pixelmix_bold.ttf"
            ))),
        );
        fonts.families.insert(
            FontFamily::Name("bold".into()),
            vec!["pixelmix_bold".to_owned()],
        );
        cc.egui_ctx.set_fonts(fonts);

        Window {
            cpu: Arc::new(Mutex::new(None)),
            ctx: Arc::new(cc.egui_ctx.clone()),
            paused: Arc::new(AtomicBool::new(false)),
            rom_loaded: false,
            clock_tx: None,

            display_texture: Arc::new(Mutex::new(display_texture)),
            _stream: stream,
            audio_queue: queue,
            input_state: Arc::new(Mutex::new(InputFlag::from_bits_truncate(0xFF))),

            options,

            menu_page: MenuPage::Main,
            logo_texture,
            arrow_texture,
            show_debug: false,
            show_color_picker: false,
        }
    }

    fn load_rom_file(path: &str) -> Vec<u8> {
        match std::fs::read(path) {
            Ok(rom_file) => rom_file,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("ROM file not found!");
                }
                panic!("{e}")
            }
        }
    }

    fn init(&mut self) {
        // Stop executor if running
        if let Some(tx) = &self.clock_tx {
            let _ = tx.send(ExecutorInstruction::Stop);
        }
        // Initialize CPU
        *self.cpu.lock().unwrap() = Some(CPU::new(
            Self::load_rom_file(&self.options.rom_path),
            &self.options,
        ));

        // Start loop on thread that receives clock signal
        // and cycles the CPU
        // Function returns the sender channel
        self.clock_tx = Some(self.start_executor());

        // Start clock
        self.start_clock();

        self.rom_loaded = true;
        self.paused.store(false, Ordering::Relaxed);
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
                        Key::Escape => {
                            if self.rom_loaded && self.paused.fetch_not(Ordering::Relaxed) {
                                self.start_clock();
                            };
                        }
                        // Toggle debug window
                        Key::F1 => self.show_debug = !self.show_debug,
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
                        // Manually activate breakpoint
                        Key::F5 => {
                            self.cpu
                                .lock()
                                .unwrap()
                                .as_ref()
                                .inspect(|cpu| cpu.breakpoint());
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
        // ctx.set_visuals(Visuals {
        //     override_text_color: Some(Color32::WHITE),
        //     ..Default::default()
        // });

        // Render the main display window
        let central_frame = egui::Frame::central_panel(&ctx.style()).inner_margin(Margin::ZERO);
        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| {
                ctx.input(|input| {
                    self.handle_input(input, true);
                });

                if self.rom_loaded {
                    // Calculate how texture should be resized on the screen to keep aspect ratio
                    let screen_size = ui.available_size();
                    let mut rect =
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(screen_size.x, screen_size.y));
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
                }

                if !self.rom_loaded || self.paused.load(Ordering::Relaxed) {
                    self.render_menu(ctx, ui);
                };
                ui.response()
            });

        if self.show_color_picker {
            self.render_color_picker(ctx);
        }

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
