use std::path::Path;

use crate::memory::{game_save::GameSave, mapper_base::{CpuRam, Mapper, Mirroring}};

pub struct InesMapper009 {
    prg_rom: Box<[u8]>,
    chr_rom: Box<[u8]>,

    prg_ram: CpuRam,

    mirroring: Mirroring,

    banks: [u8; 5],
} impl InesMapper009 {
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

            banks: [0u8; 5],
        }
    }
}

impl Mapper for InesMapper009 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                match &self.prg_ram {
                    CpuRam::Persistent(save) => {
                        save.read(addr)
                    }
                    CpuRam::Volatile(data) => {
                        if data.is_empty() {
                            let index = (addr - 0x6000) as usize & 0x07FF; 
                            data[index]
                        } else {
                            0
                        }
                    }
                }
            }
            _ => { 0 }
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x6000..=0x7FFF => {
                match &mut self.prg_ram {
                    CpuRam::Persistent(save) => {
                        save.write(addr, val)
                    }
                    CpuRam::Volatile(data) => {
                        if !data.is_empty() {
                            let index = (addr - 0x6000) as usize & 0x07FF;
                            data[index] = val;
                        }
                    }
                }
            }
            0x8000..=0xFFFF => {
                
            }
            _ => { }
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        todo!()
    }

    fn write_chr(&mut self, addr: u16, val: u8) {
        todo!()
    }

    fn mirroring(&self) -> Mirroring {
        todo!()
    }
}