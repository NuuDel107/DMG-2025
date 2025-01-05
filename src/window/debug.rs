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

    /// Renders a debug window with displays for the current state of the CPU
    pub fn render_debug(&mut self, ctx: &Context, ui: &mut Ui) {
        ctx.set_visuals(Visuals {
            override_text_color: Some(Color32::WHITE),
            ..Default::default()
        });
        let cpu_ref = self.cpu.lock().unwrap();
        Grid::new("debug_grid").min_col_width(200.0).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.monospace(format!(
                    "AF: {:#06X}   BC: {:#06X}",
                    cpu_ref.mem.read_reg_16(&Reg16::AF),
                    cpu_ref.mem.read_reg_16(&Reg16::BC)
                ));
                ui.monospace(format!(
                    "DE: {:#06X}   HL: {:#06X}",
                    cpu_ref.mem.read_reg_16(&Reg16::DE),
                    cpu_ref.mem.read_reg_16(&Reg16::HL)
                ));
                ui.monospace(format!(
                    "SP: {:#06X}   PC: {:#06X}",
                    cpu_ref.mem.read_reg_16(&Reg16::SP),
                    cpu_ref.mem.read_reg_16(&Reg16::PC)
                ));
                ui.monospace("");
                ui.monospace(format!(
                    "Z{}    N{}    H{}    C{}",
                    Self::bool_to_emoji(cpu_ref.mem.f.intersects(FlagReg::ZERO)),
                    Self::bool_to_emoji(cpu_ref.mem.f.intersects(FlagReg::SUBTRACT)),
                    Self::bool_to_emoji(cpu_ref.mem.f.intersects(FlagReg::HALF_CARRY)),
                    Self::bool_to_emoji(cpu_ref.mem.f.intersects(FlagReg::CARRY)),
                ));
                ui.monospace(format!(
                    "IME{}    HALT{}",
                    Self::bool_to_emoji(cpu_ref.mem.ime),
                    Self::bool_to_emoji(cpu_ref.halt)
                ));
                ui.monospace(format!(
                    "IF: J{} S{} T{} L{} V{}",
                    Self::bool_to_emoji(cpu_ref.mem.iflag.intersects(InterruptFlag::JOYPAD)),
                    Self::bool_to_emoji(cpu_ref.mem.iflag.intersects(InterruptFlag::SERIAL)),
                    Self::bool_to_emoji(cpu_ref.mem.iflag.intersects(InterruptFlag::TIMER)),
                    Self::bool_to_emoji(cpu_ref.mem.iflag.intersects(InterruptFlag::LCD)),
                    Self::bool_to_emoji(cpu_ref.mem.iflag.intersects(InterruptFlag::VBLANK)),
                ));
                ui.monospace(format!(
                    "IE: J{} S{} T{} L{} V{}",
                    Self::bool_to_emoji(cpu_ref.mem.ie.intersects(InterruptFlag::JOYPAD)),
                    Self::bool_to_emoji(cpu_ref.mem.ie.intersects(InterruptFlag::SERIAL)),
                    Self::bool_to_emoji(cpu_ref.mem.ie.intersects(InterruptFlag::TIMER)),
                    Self::bool_to_emoji(cpu_ref.mem.ie.intersects(InterruptFlag::LCD)),
                    Self::bool_to_emoji(cpu_ref.mem.ie.intersects(InterruptFlag::VBLANK)),
                ));
            });

            ui.vertical(|ui| {
                ui.monospace(format!(
                    "Input {}{} {:0>8b}",
                    Self::bool_to_emoji(cpu_ref.mem.io.input.select_button),
                    Self::bool_to_emoji(cpu_ref.mem.io.input.select_dpad),
                    cpu_ref.mem.io.input.flags.bits()
                ));
                ui.monospace(format!(
                    "Timer {} {}/{} %{}",
                    Self::bool_to_emoji(cpu_ref.mem.io.timer.enabled),
                    cpu_ref.mem.io.timer.counter,
                    cpu_ref.mem.io.timer.target,
                    cpu_ref.mem.io.timer.modulo
                ))
            });

            ui.vertical(|ui| {
                let next_opcode = cpu_ref.mem.read_mem(cpu_ref.mem.pc);
                let instruction = if next_opcode == 0xCB {
                    let long_opcode = cpu_ref.mem.read_mem(cpu_ref.mem.pc + 1);
                    self.instruction_db.get(long_opcode, true)
                } else {
                    self.instruction_db.get(next_opcode, false)
                };

                ui.monospace(format!("Cycles:  {}", cpu_ref.cycles));

                ui.horizontal(|ui| {
                    ui.monospace(format!("Next:    {}", &instruction.mnemonic));
                    if ui.button("?").clicked() {
                        self.show_instruction_info = !self.show_instruction_info;
                    }
                });

                let operand: String = match &instruction.bytes {
                    2 => format!("{:#04X}", cpu_ref.mem.read_mem(cpu_ref.mem.pc + 1)),
                    3 => format!("{:#06X}", cpu_ref.mem.read_mem_16(cpu_ref.mem.pc + 1)),
                    _ => "".to_string(),
                };
                ui.monospace(format!("Operand: {}", operand));

                egui::Window::new(format!("{} ({:#04X})", &instruction.mnemonic, next_opcode))
                    .open(&mut self.show_instruction_info)
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .resizable(false)
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.monospace(&instruction.desc);
                    });
            });

            ui.end_row();
        });
    }
}
