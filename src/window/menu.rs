use super::*;
use egui::{load::SizedTexture, Context, Image, ImageSource, RichText, Ui};
use std::collections::BTreeMap;
use std::path::PathBuf;

impl Window {
    pub fn render_menu(&mut self, _ctx: &Context, ui: &mut Ui) {
        let scale = self.options.window_scale as f32;
        let mut global_style = ui.style_mut().clone();

        // Define menu text styles
        use egui::FontFamily::*;
        use egui::TextStyle::*;
        let text_styles: BTreeMap<_, _> = [
            (
                Heading,
                FontId::new(scale * 12.0, egui::FontFamily::Name("bold".into())),
            ),
            (Body, FontId::new(scale * 8.0, Proportional)),
            (Button, FontId::new(scale * 8.0, Proportional)),
            (Small, FontId::new(scale * 4.0, Proportional)),
        ]
        .into();
        global_style.text_styles = text_styles;

        // Define other visual styles
        global_style.visuals.button_frame = false;
        global_style.interaction.selectable_labels = false;
        global_style.visuals.widgets.inactive.fg_stroke =
            Stroke::new(scale, Color32::from_gray(200));
        global_style.visuals.widgets.hovered.fg_stroke =
            Stroke::new(scale, Color32::from_gray(240));
        global_style.visuals.widgets.active.fg_stroke = Stroke::new(scale, Color32::from_gray(255));

        // Define styles for top bar
        let mut topbar_style = global_style.clone();
        topbar_style.text_styles.get_mut(&Button).unwrap().family =
            egui::FontFamily::Name("bold".into());

        // Define styles for reset options button
        let mut reset_style = global_style.clone();
        reset_style.visuals.widgets.inactive.fg_stroke = Stroke::new(scale, Color32::LIGHT_RED);
        reset_style.visuals.widgets.hovered.fg_stroke = Stroke::new(scale, Color32::RED);

        let global_style_arc = Arc::new(global_style);
        let topbar_style_arc = Arc::new(topbar_style);
        let reset_style_arc = Arc::new(reset_style);
        ui.set_style(global_style_arc.clone());

        let screen_size = ui.available_size();
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(screen_size.x, screen_size.y));

        ui.painter().rect(
            rect,
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(26, 29, 43, 230),
            Stroke::NONE,
        );
        egui::Frame::none()
            .inner_margin(Margin::same(20.0))
            .show(ui, |ui| {
                ui.set_style(topbar_style_arc.clone());
                ui.columns(3, |columns| {
                    columns[0].vertical_centered(|ui| {
                        if ui
                            .button(self.get_topbar_button_text(MenuPage::Main))
                            .clicked()
                        {
                            self.menu_page = MenuPage::Main;
                        }
                    });
                    columns[1].vertical_centered(|ui| {
                        if ui
                            .button(self.get_topbar_button_text(MenuPage::Options))
                            .clicked()
                        {
                            self.menu_page = MenuPage::Options;
                        }
                    });
                    columns[2].vertical_centered(|ui| {
                        if ui
                            .button(self.get_topbar_button_text(MenuPage::Info))
                            .clicked()
                        {
                            self.menu_page = MenuPage::Info;
                        }
                    });
                });

                ui.add_space(scale * 8.0);
                ui.set_style(global_style_arc.clone());

                match self.menu_page {
                    // Main page
                    MenuPage::Main => {
                        ui.columns(2, |columns| {
                            columns[0].vertical_centered(|ui| {
                                if ui.button("Load ROM  ").clicked() {
                                    if let Some(rom_path) = self.open_rom_dialog() {
                                        self.options.rom_path = rom_path.to_str().unwrap().into();
                                        self.options.save();
                                        self.init();
                                    }
                                }
                            });
                            columns[1].vertical_centered(|ui| {
                                let rom_selected = !self.options.rom_path.is_empty();
                                if ui
                                    .add_enabled(rom_selected, egui::Button::new("Reload ROM"))
                                    .clicked()
                                {
                                    self.init();
                                }
                            });
                        });
                    }
                    // Options page
                    MenuPage::Options => {
                        ui.columns(2, |columns| {
                            columns[0].vertical_centered_justified(|ui| {
                                ui.label(
                                    RichText::new("Window scale").color(Color32::from_gray(200)),
                                );
                                ui.label(RichText::new("Palette").color(Color32::from_gray(200)));
                            });
                            columns[1].vertical_centered_justified(|ui| {
                                // Window scale
                                ui.horizontal(|ui| {
                                    if self.add_arrow(ui, false).clicked()
                                        && self.options.window_scale > 2
                                    {
                                        self.options.window_scale -= 1;
                                        self.update_window();
                                    };
                                    ui.add_sized(
                                        [scale * 50.0, scale * 8.0],
                                        egui::Label::new(
                                            RichText::new(
                                                self.options.window_scale.to_string() + "X",
                                            )
                                            .color(Color32::from_gray(200)),
                                        ),
                                    );
                                    if self.add_arrow(ui, true).clicked()
                                        && self.options.window_scale < 8
                                    {
                                        self.options.window_scale += 1;
                                        self.update_window();
                                    };
                                });

                                // Palette
                                ui.horizontal(|ui| {
                                    let palette_str = match self.options.palette_preset {
                                        0 => "Original",
                                        1 => "LCD",
                                        2 => "Custom",
                                        _ => unreachable!(),
                                    };
                                    if self.add_arrow(ui, false).clicked() {
                                        if self.options.palette_preset > 0 {
                                            self.options.palette_preset -= 1;
                                        } else {
                                            self.options.palette_preset = 2;
                                        }
                                        self.update_cpu_options();
                                        self.update_display();
                                    }
                                    ui.add_sized(
                                        [scale * 50.0, scale * 8.0],
                                        egui::Label::new(
                                            RichText::new(palette_str)
                                                .color(Color32::from_gray(200)),
                                        ),
                                    );
                                    if self.add_arrow(ui, true).clicked() {
                                        if self.options.palette_preset < 2 {
                                            self.options.palette_preset += 1;
                                        } else {
                                            self.options.palette_preset = 0;
                                        }
                                        self.update_cpu_options();
                                        self.update_display();
                                    }
                                });

                                // Custom palette
                                ui.horizontal(|ui| {
                                    if self.options.palette_preset == 2
                                        && ui.button("Edit palette").clicked()
                                    {
                                        self.show_color_picker = true;
                                    }
                                });
                            });
                        });
                        ui.add_space(40.0);

                        ui.set_style(reset_style_arc.clone());
                        if ui.button("Reset options").clicked() {
                            // Don't reset ROM path
                            let rom_path = self.options.rom_path.clone();
                            self.options = Options::default();
                            self.options.rom_path = rom_path;
                            self.options.save();

                            self.update_cpu_options();
                            self.update_display();
                            self.update_window();
                        }
                        ui.set_style(global_style_arc.clone());
                    }
                    // Info page (unnecessary)
                    MenuPage::Info => {
                        ui.vertical_centered_justified(|ui| {
                            ui.add_space(scale * 20.0);
                            ui.add(
                                Image::new(ImageSource::Texture(SizedTexture::from_handle(
                                    &self.logo_texture,
                                )))
                                .fit_to_exact_size(vec2(scale * 96.0, scale * 16.0)),
                            );
                            ui.small("The world's worst Game Boy emulator");
                            ui.add_space(scale * 8.0);
                            ui.add_space(scale * 10.0);
                            // Hyperlink doesn't seem to work for some reason,
                            // so link is opened with the open crate
                            if ui.hyperlink_to("Written by NuuDel107", "").clicked() {
                                let _ = open::that("https://github.com/NuuDel107/DMG-2025");
                            }
                            ui.add_space(scale * 8.0);
                            if ui
                                .hyperlink_to(
                                    RichText::new("PixelMix font by Andrew Tyler")
                                        .font(FontId::proportional(scale * 4.0)),
                                    "",
                                )
                                .clicked()
                            {
                                let _ = open::that("https://www.dafont.com/pixelmix.font");
                            };
                        });
                        ui.allocate_space(ui.available_size());
                    }
                }
            });
    }

    pub fn render_color_picker(&mut self, ctx: &Context) {
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("color_picker_window"),
            egui::ViewportBuilder::default()
                .with_resizable(false)
                .with_maximize_button(false)
                .with_minimize_button(false)
                .with_title("Edit palette")
                .with_inner_size([600.0, 400.0]),
            |ctx, class| {
                assert!(
                    class == egui::ViewportClass::Immediate,
                    "This egui backend doesn't support multiple viewports"
                );

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.columns(2, |columns| {
                        columns[0].vertical_centered_justified(|ui| {
                            self.color_picker(ui, 0);
                            self.color_picker(ui, 2);
                        });
                        columns[1].vertical_centered_justified(|ui| {
                            self.color_picker(ui, 1);
                            self.color_picker(ui, 3);
                        });
                    });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    // Tell parent viewport that we should not show next frame:
                    self.show_color_picker = false;
                    self.options.save();
                }
            },
        );
    }

    fn color_picker(&mut self, ui: &mut Ui, index: u8) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Color {index}")).color(Color32::WHITE));
            ui.add_space(16.0);
            if ui.button("Reset").clicked() {
                *self.options.custom_palette.get_mut(index) = Palette::original().get_col(index);
                self.update_cpu_options();
                if self.paused.load(Ordering::Relaxed) {
                    self.update_display();
                }
            }
        });
        if egui::widgets::color_picker::color_picker_color32(
            ui,
            self.options.custom_palette.get_mut(index),
            egui::color_picker::Alpha::Opaque,
        ) {
            self.update_cpu_options();
            if self.paused.load(Ordering::Relaxed) {
                self.update_display();
            }
        }
    }

    fn get_topbar_button_text(&self, button_target: MenuPage) -> RichText {
        let text = match button_target {
            MenuPage::Main => "MAIN",
            MenuPage::Options => "OPTIONS",
            MenuPage::Info => "INFO",
        };
        let rich = RichText::new(text);
        if self.menu_page == button_target {
            rich.color(Color32::WHITE)
        } else {
            rich
        }
    }

    fn open_rom_dialog(&self) -> Option<PathBuf> {
        let directory = if self.options.rom_path.is_empty() {
            dirs_next::download_dir().unwrap()
        } else {
            let mut rom_path = PathBuf::from(&self.options.rom_path);
            rom_path.pop();
            rom_path
        };
        rfd::FileDialog::new()
            .set_title("Choose ROM file to load")
            .add_filter("Game Boy ROM", &["gb"])
            .add_filter("All files", &["*"])
            .set_directory(directory)
            .pick_file()
    }

    fn add_arrow(&self, ui: &mut Ui, right: bool) -> egui::Response {
        let scale = self.options.window_scale as f32;
        let angle = if right { std::f32::consts::PI } else { 0.0 };

        ui.add(egui::Button::image(
            Image::new(SizedTexture::from_handle(&self.arrow_texture))
                .fit_to_exact_size(vec2(scale * 8.0, scale * 8.0))
                .rotate(angle, vec2(0.5, 0.5)),
        ))
    }

    fn update_window(&mut self) {
        let scale = self.options.window_scale as f32;
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(
                160.0 * scale,
                144.0 * scale,
            )));
        self.options.save();
    }

    fn update_display(&mut self) {
        let cpu_option = self.cpu.lock().unwrap();
        if let Some(cpu) = cpu_option.as_ref() {
            let image = Self::get_display_texture(cpu, &self.options);
            self.display_texture
                .lock()
                .unwrap()
                .set(image, TextureOptions::NEAREST);
        }
    }

    fn update_cpu_options(&mut self) {
        if let Some(tx) = &self.clock_tx {
            let _ = tx.send(ExecutorInstruction::OptionsUpdated(self.options.clone()));
        }
    }
}
