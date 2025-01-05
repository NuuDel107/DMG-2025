use super::*;

impl Window {
    /// Logs current state of CPU to a logfile. Used to debug with https://robertheaton.com/gameboy-doctor/
    pub fn log(logfile: &mut LineWriter<File>, mem: &Memory) {
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
}