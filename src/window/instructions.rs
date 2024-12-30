use std::collections::HashMap;
use std::fs;

/// Information about a specific instruction
/// Accessed from InstructionDB using the opcode as a key
#[derive(Debug)]
pub struct InstructionInfo {
    pub mnemonic: String,
    pub desc: String,
    pub bytes: u8,
}

impl InstructionInfo {
    pub fn new(mnemonic: String, desc: String, bytes: u8) -> Self {
        Self {
            mnemonic,
            desc,
            bytes,
        }
    }
}

/// Contains information about CPU instructions, used only for debugging.
/// Instruction database from https://gist.github.com/bberak/ca001281bb8431d2706afd31401e802b
pub struct InstructionDB {
    /// Regular one byte long opcodes
    opcodes_8: HashMap<u8, InstructionInfo>,
    /// Two byte long opcodes, where the first byte is 0xCB
    opcodes_16: HashMap<u8, InstructionInfo>,
}

impl InstructionDB {
    /// Converts JSON database into a hashmap form
    pub fn init() -> Self {
        let mut opcodes_8: HashMap<u8, InstructionInfo> = HashMap::new();
        let mut opcodes_16: HashMap<u8, InstructionInfo> = HashMap::new();

        let db = fs::read_to_string("src/window/gb-instructions-db.json").unwrap();
        let json = json::parse(&db).unwrap();

        let mut i = 0;
        loop {
            let info = &json[i];
            if info.is_null() {
                break;
            }

            let mut opcode_str = info["opCode"].to_string();
            let long_opcode = &opcode_str[..2] == "CB";
            if long_opcode {
                opcode_str = opcode_str[2..].to_string();
            }

            let opcode_res = u8::from_str_radix(&opcode_str, 16);
            if let Ok(opcode) = opcode_res {
                let instruction = InstructionInfo::new(
                    info["mnemonic"].to_string(),
                    info["description"].to_string(),
                    info["bytes"].as_u8().unwrap(),
                );
                if long_opcode {
                    opcodes_16.insert(opcode, instruction);
                } else {
                    opcodes_8.insert(opcode, instruction);
                }
            }
            i += 1;
        }

        Self {
            opcodes_8,
            opcodes_16,
        }
    }

    pub fn get(&self, opcode: u8, long_opcode: bool) -> &InstructionInfo {
        if long_opcode {
            &self.opcodes_16[&opcode]
        } else {
            &self.opcodes_8[&opcode]
        }
    }
}
