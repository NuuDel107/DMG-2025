use super::cpu::memory::*;
use super::CPU;
use egui::epaint::*;

pub struct Window {
    cpu: CPU,
    window_scale: u8,
    display: [[u8; 144]; 160],
    show_debug: bool,
}

impl Window {
    pub fn new(cpu: CPU, window_scale: u8) -> Window {
        Window {
            cpu,
            window_scale,
            display: [[0; 144]; 160],
            show_debug: false,
        }
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

    fn handle_input(&mut self, input: &egui::InputState) {
        for event in &input.events {
            if let egui::Event::Key {
                key,
                pressed,
                repeat,
                ..
            } = event
            {
                if !pressed || *repeat {
                    continue;
                }
                use egui::Key;
                match *key {
                    Key::Space => self.cpu.execute(),
                    Key::F3 => self.show_debug = !self.show_debug,
                    _ => {}
                }
            }
        }
    }
}

impl eframe::App for Window {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let central_frame = egui::Frame::central_panel(&ctx.style()).inner_margin(Margin::ZERO);
        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| {
                ctx.input(|input| {
                    self.handle_input(input);
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
                            self.handle_input(input);
                        });
                        ctx.set_visuals(egui::Visuals {
                            override_text_color: Some(Color32::WHITE),
                            ..Default::default()
                        });
                        ui.monospace(format!("AF: {:#06X}", self.cpu.mem.read_reg_16(&Reg16::AF)));
                        ui.monospace(format!("BC: {:#06X}", self.cpu.mem.read_reg_16(&Reg16::BC)));
                        ui.monospace(format!("DE: {:#06X}", self.cpu.mem.read_reg_16(&Reg16::DE)));
                        ui.monospace(format!("HL: {:#06X}", self.cpu.mem.read_reg_16(&Reg16::HL)));
                        ui.monospace(format!("SP: {:#06X}", self.cpu.mem.read_reg_16(&Reg16::SP)));
                        ui.monospace(format!("PC: {:#06X}", self.cpu.mem.read_reg_16(&Reg16::PC)));
                        ui.separator();
                        ui.monospace(format!("Z: {}", self.cpu.mem.f.zero));
                        ui.monospace(format!("N: {}", self.cpu.mem.f.subtract));
                        ui.monospace(format!("H: {}", self.cpu.mem.f.half_carry));
                        ui.monospace(format!("C: {}", self.cpu.mem.f.carry));
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
