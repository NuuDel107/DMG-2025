use super::*;
use egui::{Align2, Context, Grid, Ui, Visuals};

impl Window {
    fn bool_to_emoji(bool: bool) -> String {
        if bool {
            "✔".to_string()
        } else {
            "❌".to_string()
        }
    }
    pub fn render_debug(&mut self, ctx: &Context, ui: &mut Ui) {
        ctx.set_visuals(Visuals {
            override_text_color: Some(Color32::WHITE),
            ..Default::default()
        });
        Grid::new("debug_grid").min_col_width(200.0).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.monospace(format!(
                    "AF: {:#06X}   BC: {:#06X}",
                    self.cpu.mem.read_reg_16(&Reg16::AF),
                    self.cpu.mem.read_reg_16(&Reg16::BC)
                ));
                ui.monospace(format!(
                    "DE: {:#06X}   HL: {:#06X}",
                    self.cpu.mem.read_reg_16(&Reg16::DE),
                    self.cpu.mem.read_reg_16(&Reg16::HL)
                ));
                ui.monospace(format!(
                    "SP: {:#06X}   PC: {:#06X}",
                    self.cpu.mem.read_reg_16(&Reg16::SP),
                    self.cpu.mem.read_reg_16(&Reg16::PC)
                ));
                ui.monospace("");
                ui.monospace(format!(
                    "Z{}    N{}    H{}    C{}",
                    Self::bool_to_emoji(self.cpu.mem.f.zero),
                    Self::bool_to_emoji(self.cpu.mem.f.subtract),
                    Self::bool_to_emoji(self.cpu.mem.f.half_carry),
                    Self::bool_to_emoji(self.cpu.mem.f.carry),
                ))
            });

            ui.vertical(|ui| {
                let next_opcode = self.cpu.mem.read_mem(self.cpu.mem.pc);
                let instruction = if next_opcode == 0xCB {
                    let long_opcode = self.cpu.mem.read_mem(self.cpu.mem.pc + 1);
                    self.instruction_db.get(long_opcode, true)
                } else {
                    self.instruction_db.get(next_opcode, false)
                };

                ui.horizontal(|ui| {
                    ui.monospace(format!("Next:    {}", &instruction.mnemonic));
                    if ui.button("?").clicked() {
                        self.show_instruction_info = !self.show_instruction_info;
                    }
                });

                let operand: String = match &instruction.bytes {
                    2 => format!("{:#04X}", self.cpu.mem.read_mem(self.cpu.mem.pc + 1)),
                    3 => format!("{:#06X}", self.cpu.mem.read_mem_16(self.cpu.mem.pc + 1)),
                    _ => "".to_string(),
                };
                ui.monospace(format!("Operand: {}", operand));

                egui::Window::new(&instruction.mnemonic)
                    .open(&mut self.show_instruction_info)
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.monospace(&instruction.desc);
                    });
            });

            ui.end_row();
        });
    }
}
