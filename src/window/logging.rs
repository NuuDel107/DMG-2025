use super::*;

impl Window {
    /// Logs current state of CPU to a logfile. Used to debug with https://robertheaton.com/gameboy-doctor/
    pub fn log(logfile: &mut LineWriter<File>, cpu: &std::sync::MutexGuard<CPU>) {
        let line = format!(
            "A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}\n",
            cpu.reg.a, 
            cpu.reg.f, 
            cpu.reg.b, 
            cpu.reg.c, 
            cpu.reg.d,  
            cpu.reg.e, 
            cpu.reg.h, 
            cpu.reg.l, 
            cpu.reg.sp, 
            cpu.reg.pc, 
            cpu.read(cpu.reg.pc), 
            cpu.read(cpu.reg.pc + 1), 
            cpu.read(cpu.reg.pc + 2), 
            cpu.read(cpu.reg.pc + 3)
        );
        let _ = logfile.write_all(line.as_bytes()).inspect_err(|e| eprintln!("{e}"));
    }
}