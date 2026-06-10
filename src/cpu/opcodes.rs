// json file and archive based on: https://www.nesdev.org/wiki/Visual6502wiki/6502_all_256_Opcodes
// also credits to: 
//https://gist.github.com/kirbyUK/1a0797e19f54c1e35e67ce7b385b323e
//https://github.com/michael-0acf4/opcodes-json-6502

use serde::Deserialize;

use crate::cpu::cpu::AddressingMode;
use std::{collections::HashMap, sync::{Arc, OnceLock}};

pub struct OpCode {
    pub code: u8,
    pub mnemonic: String,
    pub len:u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}
impl OpCode {
    fn new(code: u8, mnemonic: String, len: u8, cycles: u8, mode: AddressingMode) -> Self{
        OpCode {
            code: code,
            mnemonic: mnemonic,
            len: len,
            cycles: cycles,
            mode: mode,
        }
    }
}
#[derive(Deserialize)]
struct RawOpCode {
    opcode: String,
    name: String,
    bytes: u8,
    cycles: u8,
    mode: String,
}

pub static CPU_OPS_CODES: OnceLock<Arc<[OpCode]>> = OnceLock::new();
pub static OPCODES_MAP: OnceLock<HashMap<u8, &'static OpCode>> = OnceLock::new();

//lazy_static macro needed to inicialize values that need heap alocated memory as static
//such as a vector of "opcode::new()"
pub fn cpu_ops_codes() -> &'static Arc<[OpCode]> {
    CPU_OPS_CODES.get_or_init(|| {
        let json_data = include_str!("archive/6502_all_opcodes.json");
        let raw_opcodes: Vec<RawOpCode> = serde_json::from_str(json_data).expect("poorly formatted opcodes json");

        let opcodes: Vec<OpCode> = raw_opcodes
            .into_iter()
            .map(|raw| {
                let code = u8::from_str_radix(raw.opcode.trim_start_matches("$"), 16).unwrap();

                let mode = match raw.mode.as_str() {
                    "Immediate"                     => AddressingMode::Immediate,
                    "ZeroPage"                      => AddressingMode::ZeroPage,
                    "ZeroPage,X" | "ZeroPage_X"     => AddressingMode::ZeroPage_X,
                    "ZeroPage,Y" | "ZeroPage_Y"     => AddressingMode::ZeroPage_Y,
                    "Absolute"                      => AddressingMode::Absolute,
                    "Absolute,X" | "Absolute_X"     => AddressingMode::Absolute_X,
                    "Absolute,Y" | "Absolute_Y"     => AddressingMode::Absolute_Y,
                    "Indirect,X" | "Indirect_X"     => AddressingMode::Indirect_X,
                    "Indirect,Y" | "Indirect_Y"     => AddressingMode::Indirect_Y,

                    "Implied" | "Accumulator" | "Relative" | _   => AddressingMode::NoneAddressing,
                };

                OpCode::new(code, raw.name, raw.bytes, raw.cycles, mode)
            })
            .collect();
        Arc::from(opcodes)
    })
}

pub fn opcodes_map() -> &'static HashMap<u8, &'static OpCode> {
    OPCODES_MAP.get_or_init(|| {
        let mut map = HashMap::new();
        for cpline in cpu_ops_codes().iter() {
            map.insert(cpline.code, cpline);
        }
        map
    })
}