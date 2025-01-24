use super::*;
use egui::{Align2, Context, Grid, Ui};

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
        let cpu = self.cpu.lock().unwrap();
        if cpu.is_none() {
            return;
        }
        let cpu = cpu.as_ref().unwrap();
        Grid::new("debug_grid").min_col_width(200.0).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.monospace(format!(
                    "AF: {:#06X}   BC: {:#06X}",
                    cpu.reg.read_16(&Reg16::AF),
                    cpu.reg.read_16(&Reg16::BC)
                ));
                ui.monospace(format!(
                    "DE: {:#06X}   HL: {:#06X}",
                    cpu.reg.read_16(&Reg16::DE),
                    cpu.reg.read_16(&Reg16::HL)
                ));
                ui.monospace(format!(
                    "SP: {:#06X}   PC: {:#06X}",
                    cpu.reg.read_16(&Reg16::SP),
                    cpu.reg.read_16(&Reg16::PC)
                ));
                ui.monospace("");
                ui.monospace(format!(
                    "Z{}    N{}    H{}    C{}",
                    Self::bool_to_emoji(cpu.reg.f.intersects(FlagReg::ZERO)),
                    Self::bool_to_emoji(cpu.reg.f.intersects(FlagReg::SUBTRACT)),
                    Self::bool_to_emoji(cpu.reg.f.intersects(FlagReg::HALF_CARRY)),
                    Self::bool_to_emoji(cpu.reg.f.intersects(FlagReg::CARRY)),
                ));
                ui.monospace(format!(
                    "IME{}    HALT{}",
                    Self::bool_to_emoji(cpu.istate.ime),
                    Self::bool_to_emoji(cpu.halt)
                ));
                ui.monospace(format!(
                    "IF: J{} S{} T{} L{} V{}",
                    Self::bool_to_emoji(cpu.istate.iflag.intersects(InterruptFlag::JOYPAD)),
                    Self::bool_to_emoji(cpu.istate.iflag.intersects(InterruptFlag::SERIAL)),
                    Self::bool_to_emoji(cpu.istate.iflag.intersects(InterruptFlag::TIMER)),
                    Self::bool_to_emoji(cpu.istate.iflag.intersects(InterruptFlag::LCD)),
                    Self::bool_to_emoji(cpu.istate.iflag.intersects(InterruptFlag::VBLANK)),
                ));
                ui.monospace(format!(
                    "IE: J{} S{} T{} L{} V{}",
                    Self::bool_to_emoji(cpu.istate.ie.intersects(InterruptFlag::JOYPAD)),
                    Self::bool_to_emoji(cpu.istate.ie.intersects(InterruptFlag::SERIAL)),
                    Self::bool_to_emoji(cpu.istate.ie.intersects(InterruptFlag::TIMER)),
                    Self::bool_to_emoji(cpu.istate.ie.intersects(InterruptFlag::LCD)),
                    Self::bool_to_emoji(cpu.istate.ie.intersects(InterruptFlag::VBLANK)),
                ));
            });

            ui.vertical(|ui| {
                ui.monospace(format!(
                    "Input {}{} {:0>8b}",
                    Self::bool_to_emoji(cpu.input.select_button),
                    Self::bool_to_emoji(cpu.input.select_dpad),
                    cpu.input.flags.bits()
                ));
                ui.monospace(format!(
                    "Timer {} {:#06X}/{}%{}={} ",
                    Self::bool_to_emoji(cpu.timer.enabled),
                    cpu.timer.div,
                    (0b1 << (cpu.timer.div_bit + 1)) / 4,
                    cpu.timer.tma,
                    cpu.timer.tima,
                ));
                ui.monospace(format!("PPU {} {:0>10b}", cpu.ppu.mode, cpu.ppu.control));
            });

            ui.vertical(|ui| {
                ui.monospace(format!(
                    "Next: {:04X} {:04X} {:04X} {:04X}",
                    cpu.read(cpu.reg.pc),
                    cpu.read(cpu.reg.pc.wrapping_add(1)),
                    cpu.read(cpu.reg.pc.wrapping_add(2)),
                    cpu.read(cpu.reg.pc.wrapping_add(3))
                ));
            });

            ui.end_row();
        });
    }
}
