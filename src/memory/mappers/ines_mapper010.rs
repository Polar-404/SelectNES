use std::path::Path;

use crate::memory::{
    game_save::GameSave, 
    mapper_base::{CpuRam, Mapper, Mirroring}
};

pub struct InesMapper010 {
    prg_rom: Box<[u8]>,
    chr_rom: Box<[u8]>,

    prg_ram: CpuRam,

    mirroring: Mirroring,

    bank_select: u8,
}
impl InesMapper010 {
    pub fn new<P: AsRef<Path>>(prg_rom: Box<[u8]>, chr_rom: Box<[u8]>, mirroring: Mirroring, has_save: Option<P>) -> Self {

        let prg_ram = if let Some(path) = has_save { 
            CpuRam::Persistent(GameSave::new(path))
        } else { 
            CpuRam::Volatile([0; 0x2000]) 
        };
        
        Self {
            prg_rom,
            chr_rom,

            prg_ram,

            mirroring,

            bank_select: 0,

        }
    }
}
impl Mapper for InesMapper010 {
    fn read(&self, addr: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, addr: u16, val: u8) {
        todo!()
    }

    fn read_chr(&self, addr: u16) -> u8 {
        todo!()
    }

    fn write_chr(&mut self, addr: u16, val: u8) {
        todo!()
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}