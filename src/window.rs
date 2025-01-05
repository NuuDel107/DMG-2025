use super::cpu::io::*;
use super::cpu::memory::*;
use super::CPU;
use egui::epaint::*;
use std::fs::{self, OpenOptions, File};
use std::io::prelude::*;
use std::io::LineWriter;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

mod debug;
mod instructions;
use instructions::InstructionDB;

pub struct Window {
    cpu: Arc<Mutex<CPU>>,
    ctx: Arc<Mutex<Option<egui::Context>>>,
    window_scale: u8,
    display: [[u8; 144]; 160],
    instruction_db: InstructionDB,
    has_updated: bool,
    cpu_running: Arc<AtomicBool>,
    show_debug: bool,
    show_instruction_info: bool,
}

impl Window {
    pub fn new(cpu: CPU, window_scale: u8) -> Window {
        Window {
            cpu: Arc::new(Mutex::new(cpu)),
            ctx: Arc::new(Mutex::new(None)),
            window_scale,
            display: [[0; 144]; 160],
            instruction_db: InstructionDB::init(),

            cpu_running: Arc::new(AtomicBool::new(false)),
            has_updated: false,
            show_debug: false,
            show_instruction_info: false,
        }
    }

    fn start_clock(&mut self) {
        let cpu_ref = Arc::clone(&self.cpu);
        let ctx_ref = Arc::clone(&self.ctx);
        let running_ref = Arc::clone(&self.cpu_running);

        let breakpoints: Vec<u16> = vec![];

        let _ = fs::remove_file("log.txt");
        let logfile = File::create("log.txt").unwrap();
        let mut logfile = LineWriter::new(logfile);

        thread::spawn(move || loop {
            // thread::sleep(Duration::from_millis(1));
            if !running_ref.load(Ordering::Relaxed) {
                break;
            }

            let mut cpu = cpu_ref.lock().unwrap();
            if breakpoints.contains(&cpu.mem.pc) {
                cpu.breakpoint();
                running_ref.store(false, Ordering::Release);
                break;
            }
            if cpu.cycles == 0 {
                Self::log(&mut logfile, &cpu.mem);
            }
            cpu.cycle(false);
            egui::Context::request_repaint(ctx_ref.lock().unwrap().as_ref().unwrap());
        });
    }

    fn log(logfile: &mut LineWriter<File>, mem: &Memory) {
        let line = format!(
            "A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}\n",
            mem.a, 
            mem.f, 
            mem.b, 
            mem.c, 
            mem.d,  
            mem.e, 
            mem.h, 
            mem.l, 
            mem.sp, 
            mem.pc, 
            mem.read_mem(mem.pc), 
            mem.read_mem(mem.pc + 1), 
            mem.read_mem(mem.pc + 2), 
            mem.read_mem(mem.pc + 3)
        );
        let _ = logfile.write_all(line.as_bytes()).inspect_err(|e| eprintln!("{e}"));
    }

    fn set_pixel(&mut self, x: u8, y: u8, color: u8) {
        self.display[x as usize][y as usize] = color;
    }

    fn draw_tile(&mut self, x: u8, y: u8, tile_data: [u8; 16]) {
        for tile_y in 0..8 {
            for tile_x in 0..8 {
                let a = tile_data[usize::from(tile_y * 2)].reverse_bits() & (1 << tile_x) != 0;
                let b = tile_data[usize::from(tile_y * 2 + 1)].reverse_bits() & (1 << tile_x) != 0;

                let mut color: u8 = 0;
                if a && b {
                    color = 3;
                } else if a {
                    color = 1;
                } else if b {
                    color = 2;
                }

                self.set_pixel(x + tile_x, y + tile_y, color);
            }
        }
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

                if *repeat {
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
                        self.cpu.lock().unwrap().update_input(input, *pressed);
                    }
                }

                if *pressed {
                    match *key {
                        Key::Space => {
                            if !self.cpu_running.fetch_not(Ordering::Relaxed) {
                                self.start_clock();
                            };
                        }
                        Key::F1 => self.show_debug = !self.show_debug,
                        Key::F3 => {
                            if !self.cpu_running.load(Ordering::Relaxed) {
                                self.cpu.lock().unwrap().cycle(true)
                            }
                        }
                        Key::F5 => {
                            self.cpu.lock().unwrap().breakpoint();
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
        if !self.has_updated {
            self.has_updated = true;

            self.ctx = Arc::new(Mutex::new(Some(ctx.clone())));
            if self.cpu_running.load(Ordering::Relaxed) {
                self.start_clock();
            }
        }

        let central_frame = egui::Frame::central_panel(&ctx.style()).inner_margin(Margin::ZERO);
        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| {
                ctx.input(|input| {
                    self.handle_input(input, true);
                });
                // Paint background white (empty color)
                let (_id, screen_rect) = ui.allocate_space(ui.available_size());
                let background =
                    RectShape::new(screen_rect, Rounding::ZERO, Color32::WHITE, Stroke::NONE);

                let mut pixels = vec![Shape::Rect(background)];
                for x in 0..160 {
                    for y in 0..144 {
                        // Loop through display table
                        let pixel = self.display[x][y];
                        let color = match pixel {
                            1 => Color32::GRAY,
                            2 => Color32::DARK_GRAY,
                            3 => Color32::BLACK,
                            // Don't bother drawing empty pixels (color 0)
                            // since background is already white
                            _ => continue,
                        };
                        // Create shape representing pixel
                        let scale = self.window_scale as f32;
                        let pos = Pos2::new(x as f32 * scale, y as f32 * scale);
                        let rect = Rect::from_min_size(pos, vec2(scale, scale));
                        let pixel = RectShape::new(rect, Rounding::ZERO, color, Stroke::NONE);
                        pixels.push(Shape::Rect(pixel));
                    }
                }
                // Paint all pixels
                ui.painter().extend(pixels);
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
