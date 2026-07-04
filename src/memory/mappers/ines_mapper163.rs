use crate::engine::terminal::struct_terminal::{LogType, print_logs};
use crate::memory::mapper_base::*;

use crate::memory::game_save::GameSave;

pub struct InesMapper163 {
    game_save: GameSave,

    prg_rom: Box<[u8]>,
    chr_rom: Box<[u8]>,
    chr_ram: Box<[u8]>,

    mirroring: Mirroring,

    reg_5000: u8,
    reg_5100: u8,
    reg_5101: u8,
    reg_5200: u8,
    reg_5300: u8,
    
    last_a13: bool,
    trigger_bit: bool,

    auto_chr_switch: bool,
    latched_a9: u16,
}
impl InesMapper163 {
    pub fn new(prg_rom: Box<[u8]>, chr_rom: Box<[u8]>, mirroring: Mirroring, game_save: GameSave) -> Self {
        let chr_ram = if chr_rom.is_empty() { vec![0; 8192].into() } else { vec![].into() };
        Self {
            game_save,
            prg_rom,
            chr_rom,
            chr_ram,

            mirroring,

            reg_5000: 0u8,
            reg_5100: 0,
            reg_5101: 0,
            reg_5200: 0u8,
            reg_5300: 1,
            
            last_a13: false,
            trigger_bit: true,

            auto_chr_switch: false,
            latched_a9: 0,
        }
    }
    fn prg_bank(&self) -> usize {
        let mut bank = ((self.reg_5200 & 0x0F) as usize) << 6 | ((self.reg_5000 & 0x3F) as usize);
        if (self.reg_5300 & 0x01) == 0 {
            bank |= 0x03; 
        }
        bank
    }
}

impl Mapper for InesMapper163 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x5000..=0x5FFF => {
                let open_bus = (addr >> 8) as u8; 
                
                let mut val = match addr {
                    0x5500 | 0x5501 => {
                        let latch = self.reg_5100 & 0x0B;
                        let trigger = if self.trigger_bit { 0x04 } else { 0x00 };
                        
                        latch | trigger | (open_bus & 0xF0)
                    }
                    _ => open_bus
                };

                if (self.reg_5300 & 0x02) != 0 {
                    val = (val & !0x03) | ((val & 1) << 1) | ((val >> 1) & 1);
                }
                val
            }
            0x6000..=0x7FFF => self.game_save.read(addr),
            0x8000..=0xFFFF => {
                let offset = (self.prg_bank() * 0x8000) + (addr as usize - 0x8000);
                self.prg_rom[offset % self.prg_rom.len()]
            }
            _ => 0
        }
    }

    fn write(&mut self, addr: u16, mut val: u8) {
        if (0x5000..=0x5200).contains(&addr) && (self.reg_5300 & 0x02) != 0 {
            val = (val & !0x03) | ((val & 1) << 1) | ((val >> 1) & 1);
        }

        match addr {
            0x5000 => {
                self.reg_5000 = val;
                self.auto_chr_switch = (val & 0x80) != 0;
            }

            // 0x5100 => { self.reg_5100 = val; print_logs(LogType::Warning,format!("WR 5100: {:02X}", val)) },
            // 0x5101 => { self.reg_5101 = val; print_logs(LogType::Warning,format!("WR 5101: {:02X}", val)) },
            0x5100 => {
                if (self.reg_5100 & 0x01) == 1 && (val & 0x01) == 0 {
                    self.trigger_bit = !self.trigger_bit;
                }
                self.reg_5100 = val;
                print_logs(LogType::Info,format!("WR 5100: {:02X}", val))
            }
            0x5101 => {
                if (self.reg_5101 & 0x01) == 1 && (val & 0x01) == 0 {
                    self.trigger_bit = !self.trigger_bit;
                }
                self.reg_5101 = val;
                print_logs(LogType::Info,format!("WR 5101: {:02X}", val));
            }

            0x5200 => self.reg_5200 = val,
            0x5300 => self.reg_5300 = val,
            0x6000..=0x7FFF => self.game_save.write(addr, val),
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let mut mapped_addr = addr;

        if self.auto_chr_switch {
            mapped_addr = (addr & 0x0FFF) | self.latched_a9;
        }

        if !self.chr_rom.is_empty() {
            self.chr_rom[mapped_addr as usize]
        } else {
            self.chr_ram[mapped_addr as usize]
        }
    }

    fn write_chr(&mut self, addr: u16, val: u8) {
        if self.chr_rom.is_empty() {
            let mut mapped_addr = addr;
            
            if self.auto_chr_switch {
                mapped_addr = (addr & 0x0FFF) | self.latched_a9;
            }
        
            self.chr_ram[mapped_addr as usize] = val;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
    fn notify_ppu_address(&mut self, addr: u16) {
        let current_a13 = (addr & 0x2000) != 0;
        
        if current_a13 && !self.last_a13 {
            self.latched_a9 = if (addr & 0x0200) != 0 { 0x1000 } else { 0x0000 };
        }
        
        self.last_a13 = current_a13;
    }
}
