use super::*;
use egui::{Align2, Context, Grid, ImageSource, RichText, Style, Ui};
use std::collections::BTreeMap;
use std::path::PathBuf;

impl Window {
    pub fn render_menu(&mut self, ctx: &Context, ui: &mut Ui) {
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
        global_style.visuals.widgets.inactive.fg_stroke =
            Stroke::new(scale, Color32::from_gray(200));
        global_style.visuals.widgets.hovered.fg_stroke =
            Stroke::new(scale, Color32::from_gray(240));
        global_style.visuals.widgets.active.fg_stroke = Stroke::new(scale, Color32::from_gray(255));

        // Define styles for top bar
        let mut topbar_style = global_style.clone();
        topbar_style.text_styles.get_mut(&Button).unwrap().family =
            egui::FontFamily::Name("bold".into());

        let global_style_arc = Arc::new(global_style);
        let topbar_style_arc = Arc::new(topbar_style);
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
                    MenuPage::Info => {
                        ui.vertical_centered_justified(|ui| {
                            ui.add_space(scale * 32.0);
                            ui.add(
                                egui::Image::new(ImageSource::Texture(
                                    egui::load::SizedTexture::from_handle(&self.logo_texture),
                                ))
                                .fit_to_exact_size(vec2(scale * 96.0, scale * 16.0)),
                            );
                            ui.small("The world's worst Game Boy emulator");
                            ui.add_space(scale * 8.0);
                            ui.horizontal(|ui| {
                                ui.add_space(scale * 20.0);
                                ui.label(RichText::new("Written by ").color(Color32::WHITE));
                                ui.add(
                                    egui::Hyperlink::from_label_and_url(
                                        "NuuDel107",
                                        "https://github.com/NuuDel107/DMG-2025",
                                    )
                                    .open_in_new_tab(true),
                                );
                            });
                        });
                        ui.allocate_space(ui.available_size());
                    }
                    _ => {}
                }
            });
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
}
