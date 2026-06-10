use crate::memory::mapper_base::*;

use crate::memory::game_save::GameSave;

/// http://nesdev.org/wiki/MMC3
/// 
/// ### BANKS
/// 
/// CPU **$6000-$7FFF** 8 KB PRG RAM bank (optional)
/// 
/// CPU **$8000-$9FFF** (or $C000-$DFFF): 8 KB switchable PRG ROM bank
/// 
/// CPU **$A000-$BFFF** 8 KB switchable PRG ROM bank
/// 
/// CPU **$C000-$DFFF** (or $8000-$9FFF): 8 KB PRG ROM bank, fixed to the second-last bank
/// 
/// CPU **$E000-$FFFF** 8 KB PRG ROM bank, fixed to the last bank
/// 
/// PPU **$0000-$07FF** (or $1000-$17FF): 2 KB switchable CHR bank
/// 
/// PPU **$0800-$0FFF** (or $1800-$1FFF): 2 KB switchable CHR bank
/// 
/// PPU **$1000-$13FF** (or $0000-$03FF): 1 KB switchable CHR bank
/// 
/// PPU **$1400-$17FF** (or $0400-$07FF): 1 KB switchable CHR bank
/// 
/// PPU **$1800-$1BFF** (or $0800-$0BFF): 1 KB switchable CHR bank
/// 
/// PPU **$1C00-$1FFF** (or $0C00-$0FFF): 1 KB switchable CHR bank
pub struct InesMapper004 {

    /// $6000-$7FFF
    game_save: GameSave,


    prg_rom: Box<[u8]>,
    chr_rom: Box<[u8]>,
    chr_ram: Box<[u8]>,

    mirroring: Mirroring,

    bank_select_register: u8,

    /// Array dos 8 registradores de banco do MMC3.
    /// O registrador a ser escrito é selecionado pelos bits RRR do Bank Select Register:
    ///
    /// ```text
    /// 7  bit  0
    /// ---- ----
    /// CPMx xRRR
    /// |||   |||
    /// |||   +++- Specify which bank register to update on next write to Bank Data register
    /// |||          000: R0: Select 2 KB CHR bank at PPU $0000-$07FF (or $1000-$17FF)
    /// |||          001: R1: Select 2 KB CHR bank at PPU $0800-$0FFF (or $1800-$1FFF)
    /// |||          010: R2: Select 1 KB CHR bank at PPU $1000-$13FF (or $0000-$03FF)
    /// |||          011: R3: Select 1 KB CHR bank at PPU $1400-$17FF (or $0400-$07FF)
    /// |||          100: R4: Select 1 KB CHR bank at PPU $1800-$1BFF (or $0800-$0BFF)
    /// |||          101: R5: Select 1 KB CHR bank at PPU $1C00-$1FFF (or $0C00-$0FFF)
    /// |||          110: R6: Select 8 KB PRG ROM bank at $8000-$9FFF (or $C000-$DFFF)
    /// |||          111: R7: Select 8 KB PRG ROM bank at $A000-$BFFF
    /// ||+-------- Nothing on the MMC3, see MMC6
    /// |+--------- PRG ROM bank mode (0: $8000-$9FFF swappable, $C000-$DFFF fixed to second-last bank;
    /// |                              1: $C000-$DFFF swappable, $8000-$9FFF fixed to second-last bank)
    /// +---------- CHR A12 inversion (0: two 2 KB banks at $0000-$0FFF, four 1 KB banks at $1000-$1FFF;
    ///                                1: two 2 KB banks at $1000-$1FFF, four 1 KB banks at $0000-$0FFF)
    /// ```
    bank_registers: [u8; 8],
    bank_select: usize, 

    irq_counter: u8,
    irq_latch: u8,
    irq_enabled: bool,
    irq_pending: bool,
    irq_reload: bool,

    last_a12: bool,
    a12_low_counter: u32,

    prg_ram_chip_enable: bool,     
    prg_ram_w_protection: bool,
}

impl InesMapper004 {
    pub fn new(prg_rom: Box<[u8]>, chr_rom: Box<[u8]>, mirroring: Mirroring, game_save: GameSave) -> Self {
        let chr_ram = if chr_rom.is_empty() { vec![0; 8192].into() } else { vec![].into() };

        InesMapper004 {
            game_save,
            prg_rom,
            chr_rom,
            chr_ram,

            mirroring,

            bank_select_register: 0,
            bank_registers: [0; 8],
            bank_select: 0,

            //IRQ
            irq_counter: 0,
            irq_latch: 0,

            irq_enabled: false,
            irq_pending: false,
            irq_reload: false,  

            last_a12: false,
            a12_low_counter: 0,

            prg_ram_chip_enable: false,
            prg_ram_w_protection: false,
        }
    }
    fn bank_switch(&self, addr: u16) -> usize {
        let prg_mode = (self.bank_select_register & 0x40) != 0;
        let total_banks = self.prg_rom.len() / 8192;

        match addr {
            0x8000..=0x9FFF => {
                if !prg_mode {
                    (self.bank_registers[6] as usize) * 8192 + (addr as usize & 0x1FFF)
                } else {
                    (total_banks - 2) * 8192 + (addr as usize & 0x1FFF)
                }
            }
            0xA000..=0xBFFF => {
                // R7 always maps here, in both modes
                (self.bank_registers[7] as usize) * 8192 + (addr as usize & 0x1FFF)
            }
            0xC000..=0xDFFF => {
                if !prg_mode {
                    (total_banks - 2) * 8192 + (addr as usize & 0x1FFF)
                } else {
                    (self.bank_registers[6] as usize) * 8192 + (addr as usize & 0x1FFF)
                }
            }
            0xE000..=0xFFFF => {
                (total_banks - 1) * 8192 + (addr as usize & 0x1FFF)
            }
            _ => 0
        }
    }

    fn _get_chr_offset(&self, reg_idx: usize, addr: u16, size_kb: usize) -> usize {
        let bank = self.bank_registers[reg_idx] as usize;
        if size_kb == 1 {
            (bank * 1024) + (addr as usize & 0x03FF)
        } else{       
            ((bank & 0xFE) * 1024) + (addr as usize & 0x07FF)
        }
    }

    fn bank_switch_chr(&self, addr: u16) -> usize {
        let chr_mode = self.bank_select_register & 0x80 != 0;

        if !chr_mode {
            // Mode 0
            match addr {
                0x0000..=0x07FF => self._get_chr_offset(0, addr, 2),
                0x0800..=0x0FFF => self._get_chr_offset(1, addr, 2),
                0x1000..=0x13FF => self._get_chr_offset(2, addr, 1),
                0x1400..=0x17FF => self._get_chr_offset(3, addr, 1),
                0x1800..=0x1BFF => self._get_chr_offset(4, addr, 1),
                0x1C00..=0x1FFF => self._get_chr_offset(5, addr, 1),
                _ => 0
            }
        } else {
            // Mode 1
            match addr {
                0x0000..=0x03FF => self._get_chr_offset(2, addr, 1),
                0x0400..=0x07FF => self._get_chr_offset(3, addr, 1),
                0x0800..=0x0BFF => self._get_chr_offset(4, addr, 1),
                0x0C00..=0x0FFF => self._get_chr_offset(5, addr, 1),
                0x1000..=0x17FF => self._get_chr_offset(0, addr, 2),
                0x1800..=0x1FFF => self._get_chr_offset(1, addr, 2),
                _ => 0
            }
        }
    }
}

impl Mapper for InesMapper004 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                self.game_save.read(addr)
            }
            0x8000..=0xFFFF => {
                //calculates the actual offset within the ROM based on the MMC3 registers
                let mapper_offset = self.bank_switch(addr);

                
                // Security/Mirroring: The '%' operator ensures that the index never exceeds
                // the size of the loaded PRG_ROM, preventing 'out of bounds' panic
                // if the game requests a non-existent bank, it 'mirrors' it back to the beginning
                self.prg_rom[mapper_offset % self.prg_rom.len()]
            }
            _ => {0}
        }
    }

    /// *REGISTERS:* https://www.nesdev.org/wiki/MMC3#Registers
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_chip_enable && !self.prg_ram_w_protection {
                    self.game_save.write(addr, val);
                }
            }
            0x8000..=0x9FFF => {
                //even addr selects the bank register
                if addr % 2 == 0 {
                    self.bank_select_register = val;
                    self.bank_select = (val & 0x07) as usize;
                } 
                //odd addr will write at selected bank registers
                else {
                    self.bank_registers[self.bank_select] = val;
                }
            }
            0xA000..=0xBFFF => {
                //Nametable arrangement ($A000-$BFFE, even)
                if addr % 2 == 0 {
                    if val & 0x01 == 0 {
                        self.mirroring = Mirroring::Vertical;
                    } else {
                        self.mirroring = Mirroring::Horizontal;
                    }
                }
                //PRG RAM protect ($A001-$BFFF, odd)
                else {
                    //Write protection (0: allow writes; 1: deny writes)
                    self.prg_ram_w_protection = (val & 0b0100_0000) != 0;

                    //PRG RAM chip enable (0: disable; 1: enable)
                    self.prg_ram_chip_enable = (val & 0b1000_0000) != 0;
                }
            }
            0xC000..=0xDFFF => {
                //IRQ latch ($C000-$DFFE, even)
                if addr % 2 == 0 {
                    self.irq_latch = val
                }
                //IRQ reload ($C001-$DFFF, odd)
                else {
                    self.irq_counter = 0;
                    self.irq_reload = true;
                }
            }
            0xE000..=0xFFFF => {
                //IRQ disable ($E000-$FFFE, even)
                //Writing any value to this register will disable MMC3 interrupts AND acknowledge any pending interrupts.
                if addr % 2 == 0 {
                    self.irq_enabled = false;
                    self.acknowledge_irq();
                }
                //IRQ enable ($E001-$FFFF, odd)
                //Writing any value to this register will enable MMC3 interrupts.
                else {
                    self.irq_enabled = true;
                }
            }

            _ => {} //ignores writing
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        let mapper_offset = self.bank_switch_chr(addr);

        if self.chr_rom.is_empty() {
            self.chr_ram[mapper_offset % self.chr_ram.len()]
        } else {
            self.chr_rom[mapper_offset % self.chr_rom.len()]
        }
    }

    fn write_chr(&mut self, addr: u16, val: u8) {
        if self.chr_rom.is_empty() {
            let mapper_offset = self.bank_switch_chr(addr);

            self.chr_ram[mapper_offset % self.chr_ram.len()] = val;
        }
        //ignores writing at chr_rom
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    #[inline]
    fn acknowledge_irq(&mut self) {
        self.irq_pending = false
    }

    fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    fn notify_ppu_address(&mut self, addr: u16) {
        let a12 = addr & 0x1000 != 0;

        if !a12 {
            self.a12_low_counter += 1;
        } else {
            if !self.last_a12 {
                if self.irq_counter == 0 || self.irq_reload {
                    self.irq_counter = self.irq_latch;
                    self.irq_reload = false;
                } else {
                    self.irq_counter -= 1;
                }

                if self.irq_counter == 0 && self.irq_enabled {
                    self.irq_pending = true;
                }
            }
            self.a12_low_counter = 0;
        }

        self.last_a12 = a12;
    }
}