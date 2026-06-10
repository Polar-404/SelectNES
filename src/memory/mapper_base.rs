use crate::memory::game_save::GameSave;

#[derive(Clone, Copy, Debug)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    SingleScreenLower,
    SingleScreenUpper
}

pub enum CpuRam {
    Volatile([u8; 0x2000]),
    Persistent(GameSave),
}

/// A memory mapper that abstracts over different NES cartridge board configurations.
///
/// NES cartridges use various mapper chips to extend the addressable memory beyond
/// the CPU's and PPU's native limits. Each mapper implements a different bank-switching
/// strategy for PRG ROM (program data) and CHR ROM (graphics data).
///
/// Implementing this trait allows the emulator to treat all cartridges uniformly,
/// regardless of their underlying mapper chip (e.g., NROM, MMC1, MMC3).
pub trait Mapper {
    fn read(&self, addr: u16) -> u8;

    fn write(&mut self, addr: u16, val: u8);
    
    fn read_chr(&self, addr: u16) -> u8;
    
    fn write_chr(&mut self, addr: u16, val: u8);

    fn mirroring(&self) -> Mirroring;

    //optional (depends on the cartridge)
    fn irq_pending(&self) -> bool { false }
    fn acknowledge_irq(&mut self) {}
    fn notify_ppu_address(&mut self, _addr: u16) {}
}