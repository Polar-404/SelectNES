use std::{cell::Cell, path::Path};

use crate::memory::{game_save::GameSave, mapper_base::{CpuRam, Mapper, Mirroring}};

pub struct InesMapper009 {
    prg_rom: Box<[u8]>,
    chr_rom: Box<[u8]>,

    prg_ram: CpuRam,

    mirroring: Mirroring,

    ///nesdev.org/wiki/MMC2#Registers
    banks: [u8; 5],

    latch_0: Cell<u8>,
    latch_1: Cell<u8>,
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

            latch_0: Cell::new(0xFD),
            latch_1: Cell::new(0xFD),
        }
    }
    fn update_latch(&self, addr: u16) {
        match addr {
            0x0FD8 => self.latch_0.set(0xFD),
            0x0FE8 => self.latch_0.set(0xFE),
            0x1FD8..=0x1FDF => self.latch_1.set(0xFD),
            0x1FE8..=0x1FEF => self.latch_1.set(0xFE),
            _ => {}
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
                        if !data.is_empty() {
                            let index = (addr - 0x6000) as usize & 0x1FFF; 
                            data[index]
                        } else {
                            0
                        }
                    }
                }
            }
            0x8000..=0x9FFF => {
                //switchable prg rom bank
                let bank_offset = (self.banks[0] as usize) * 0x2000;
                let rom_offset = (addr - 0x8000) as usize;
                self.prg_rom[bank_offset + rom_offset]
            }
            0xA000..=0xFFFF => {
                //24kb fixed to the last 3 banks
                let start_of_last_24kb = self.prg_rom.len() - (3 * 0x2000);
                let rom_offset = (addr - 0xA000) as usize;
                self.prg_rom[start_of_last_24kb + rom_offset]
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
                            let index = (addr - 0x6000) as usize & 0x1FFF;
                            data[index] = val;
                        }
                    }
                }
            }

            //PRG ROM bank select ($A000-$AFFF)
            0xA000..=0xAFFF => {
                self.banks[0] = val & 0b0000_1111
            }

            0xB000..=0xBFFF => self.banks[1] = val & 0x1F, // Select 4 KB CHR ROM bank for PPU $0000-$0FFF (Latch 0 = $FD)
            0xC000..=0xCFFF => self.banks[2] = val & 0x1F, // Select 4 KB CHR ROM bank for PPU $0000-$0FFF (Latch 0 = $FE)
            0xD000..=0xDFFF => self.banks[3] = val & 0x1F, // Select 4 KB CHR ROM bank for PPU $1000-$1FFF (Latch 1 = $FD)
            0xE000..=0xEFFF => self.banks[4] = val & 0x1F, // Select 4 KB CHR ROM bank for PPU $1000-$1FFF (Latch 1 = $FE)
            
            //Mirroring ($F000-$FFFF)
            0xF000..=0xFFFF => {
                self.mirroring = if val & 0x01 == 0 { Mirroring::Vertical } else { Mirroring::Horizontal }
            }
            _ => { }
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        self.update_latch(addr);


        // PPU $0000-$0FFF: Two 4 KB switchable CHR ROM banks
        // PPU $1000-$1FFF: Two 4 KB switchable CHR ROM banks
        let bank_index = match addr {
            0x0000..=0x0FFF => {
                if self.latch_0.get() == 0xFD {
                    self.banks[1]
                } else {
                    self.banks[2]
                }
            }
            0x1000..=0x1FFF => {
                if self.latch_1.get() == 0xFD {
                    self.banks[3]
                } else {
                    self.banks[4]
                }
            }
            _ => 0,
        };

        let chr_offset = (bank_index as usize * 4096) + (addr as usize % 4096);
        self.chr_rom[chr_offset]
    }

    fn write_chr(&mut self, _addr: u16, _val: u8) { }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}