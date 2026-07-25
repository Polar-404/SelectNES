use std::path::Path;

use crate::engine::terminal::struct_terminal::{self, LogType};
use crate::memory::{mappers, mapper_base::*};

use crate::{
    apu::apu::APU,
    apu::audio::AudioOutput,
    memory::game_save::GameSave,
    ppu::ppu::PPU,
    memory::joypads::JoyPad,
};

use ringbuf::traits::{Observer as _, Producer as _};

use std::rc::Rc;
use std::cell::RefCell;

pub struct TickResult {
    pub nmi: bool,
    pub irq: bool,
}

pub struct BUS {

    //[https://www.nesdev.org/wiki/CPU_memory_map]

    cpu_memory: [u8; 0x0800],
    //nes_apu_and_io_registers: [u8; 0x18],

    ///APU and I/O functionality that is normally disabled.
    apu_and_io_functionality: [u8; 0x08], 

    pub joypad_1: JoyPad,
    pub joypad_2: JoyPad,

    ///Unmapped. Available for cartridge use.
    ///[$6000–$7FFF | Usually cartridge RAM, when present]
    ///[$8000–$FFFF | Usually cartridge ROM and mapper registers]
    pub mapper: Rc<RefCell<dyn Mapper>>,
    pub ppu: PPU,
    pub apu: APU,
}

impl BUS {
    
    pub fn new(mapper: Rc<RefCell<dyn Mapper>>) -> Self {
        BUS {
            cpu_memory: [0; 0x0800],
            apu_and_io_functionality: [0; 0x08],
            joypad_1: JoyPad::new(),
            joypad_2: JoyPad::new(),
            mapper: Rc::clone(&mapper),
            ppu: PPU::new(mapper),
            apu: APU::default(),
        }
    }

    pub fn peek(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                let addr = addr & 0x07FF;
                self.cpu_memory[addr as usize]
            }
            0x2000..=0x3FFF => {
                self.ppu.peek(addr)
            }
            0x4000..=0x4014 => {
                0
            }
            0x4015 => self.apu.read_status(),
            0x4016 => {
                self.joypad_1.peek()
            }
            0x4017 => {
                self.joypad_2.peek()
            }
            0x4018..=0x401F => {
                let addr = addr - 0x4018;
                self.apu_and_io_functionality[addr as usize]
            }
            0x4020..=0xFFFF => {
                self.mapper.borrow().read(addr)
                //self.unmapped[addr as usize]
            }
        }
    }

    #[inline(always)]
    pub fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                let addr = addr & 0x07FF;
                self.cpu_memory[addr as usize]
            }
            0x2000..=0x3FFF => {
                let addr: u8 = (addr & 0x07) as u8;
                self.ppu.read_registers(addr)
            }
            0x4000..=0x4014 => {
                0
            }
            0x4015 => self.apu.read_status(),
            0x4016 => {
                self.joypad_1.read()
            }
            0x4017 => {
                self.joypad_2.read()
            }
            0x4018..=0x401F => {
                let addr = addr - 0x4018;
                self.apu_and_io_functionality[addr as usize]
            }
            0x4020..=0xFFFF => {
                self.mapper.borrow().read(addr)
                //self.unmapped[addr as usize]
            }
        }
    }

    ///Returns true if the cpu should trigger an NMI
    /// 
    ///It should trigger an NMI if the ppu writes at ppuctrl AND NMI was just enabled AND the PPU is already in vblank
    #[inline(always)]
    pub fn mem_write(&mut self, addr: u16, val: u8) -> bool {
        match addr {
            //cpu ram
            0x0000..=0x1FFF => {
                let addr = addr & 0x07FF;
                self.cpu_memory[addr as usize] = val;
                false
            }
            //ppu registers
            0x2000..=0x3FFF => {
                let addr = addr & 0x0007;
                if self.ppu.write_registers(addr, val) {
                    return true
                }
                false
            }
            // dma, controls and audio
            0x4000..=0x4017 => {

                if addr == 0x4014 {
                    // val é a página — ex: 0x02 significa $0200-$02FF
                    let page_start = (val as u16) << 8;
                    let mut page = [0u8; 256];
                    for i in 0..256u16 {
                        page[i as usize] = self.mem_read(page_start + i);
                    }
                    self.ppu.oam_dma_write(&page);
                    return false;
                }

                if addr == 0x4016 {
                    self.joypad_1.write(val);
                    self.joypad_2.write(val);
                    return false
                }
                
                self.apu.write_register(addr, val);
                false
            }
            // *currently disabled* apu and yo functionality
            0x4018..=0x401F => {
                let addr = addr - 0x4018;
                self.apu_and_io_functionality[addr as usize] = val;
                false
            }
            //cartridge
            0x4020..=0xFFFF => {
                //passing it's real address(without subtraction) to the mapper to take care of it
                self.mapper.borrow_mut().write(addr, val);
                false
            }
        }
    }

    pub fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        return (hi << 8) | (lo as u16);
    }

    pub fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8; 
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }

    pub fn nmi_active(&self) -> bool {
        self.ppu.nmi_active()
    }

    pub fn tick(&mut self, cycles: u8) -> TickResult {
        self.ppu.tick(cycles as u16 * 3);
        for _ in 0..cycles {
            self.apu.step();
        }

        let mut tick_result = TickResult {
            nmi: false,
            irq: false,
        };

        if self.ppu.nmi_occurred {
            self.ppu.nmi_occurred = false;
            tick_result.nmi = true;
        }

        tick_result.irq = self.mapper.borrow().irq_pending();

        tick_result
    }

    pub fn sync_audio(&mut self, cycles: u8, audio: &mut (AudioOutput, u32)) {
        let capacity = audio.0.producer.capacity().get() as f64;
        let len = audio.0.producer.occupied_len() as f64;
        let fullness = len / capacity;

        self.apu.tick(cycles, audio.1, fullness, |sample: f32| {
            let _ = audio.0.producer.try_push(sample);
        });
    }
    
    //pub fn load(&mut self, program: Vec<u8>) {
    //    self.unmapped[0x0000 .. (0x0000 + program.len())].copy_from_slice(&program[..]); //copia de src: program para self: memory
    //    self.mem_write_u16(0xFFFC,0x8000);
    //}
}

/// fn to reduce code repetition
fn wrap_in_pointers<T>(mapper: T) ->  Rc<RefCell<dyn Mapper>>
where T: Mapper + 'static {
    Rc::new(
        RefCell::new(
            mapper
        )
    )
}

const NES_SIGNATURE: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];

/// Loads an iNES ROM file and returns the appropriate "mapper" for the cartridge.
/// 
/// The "Mappers" in this codebase are a customized 'struct/data format' with all the data of the cartridge on it
/// (which ik isn't the real meaning of an actual NES mapper)
/// organized in a format that this emulator can read
///
/// Parses the 16-byte iNES header to extract ROM layout information, slices the
/// binary data into PRG and CHR regions, determines the mirroring mode, and
/// constructs the mapper instance corresponding to the cartridge's mapper ID.
///
/// ### iNES Header Format
///
/// ```text
/// Offset  Size  Description
/// ──────────────────────────────────────────────────────────────────────
/// 0–3     4     Magic: $4E $45 $53 $1A ("NES" + MS-DOS EOF marker)
/// 4       1     PRG ROM size in 16 KB units
/// 5       1     CHR ROM size in 8 KB units (0 = board uses CHR RAM)
/// 6       1     Flags 6: [Mapper low nibble | 4-screen | trainer | battery | mirroring]
/// 7       1     Flags 7: [Mapper high nibble | NES 2.0 | PlayChoice | VS Unisystem]
/// 8       1     PRG RAM size (rarely used)
/// 9       1     TV system (rarely used)
/// 10      1     TV system / PRG RAM presence (unofficial)
/// 11–15   5     Padding (should be zero)
///
/// Flags 6 bit layout:
///   7 6 5 4   3         2        1          0
///   ─────────────────────────────────────────
///   Mapper lo │ 4-screen │ trainer │ battery │ mirroring
///                                            └─ 0: horizontal arrangement
///                                               1: vertical arrangement
/// ```
///
/// ### Arguments
///
/// * `path` - Path to the `.nes` ROM file.
///
/// ### Errors
///
/// Returns an error if:
/// - The file cannot be read.
/// - The mapper ID extracted from the header is not yet implemented.
/// - Four-screen mirroring is encountered (currently unimplemented).
/// 
/// **For more information about real NES Mappers, go to:** https://www.nesdev.org/wiki/Mapper
pub fn load_rom_from_file(path: &Path) -> Result<Rc<RefCell<dyn Mapper>>, Box<dyn std::error::Error>> {

    //reads the entire content of a file into a vector of bytes(which is excatly what i need)
    let rom_data = std::fs::read(path)?;
    let header = &rom_data[0..16];

    let mapper_match = (header[7] & 0xF0) | (header[6] >> 4);

    let has_trainer = (header[6] & 0b0000_0100) != 0;

    if header[0..4] != NES_SIGNATURE {
        struct_terminal::print_logs(
            LogType::Warning, 
            format!("THE ROM HEADER DOESN'T HAVE A NES SIGNATURE, THIS ROM MIGHT BE INVALID")
        );
    }

    struct_terminal::print_logs(LogType::Info, format!("--- ROM HEADER INFO ---"));
    struct_terminal::print_logs(LogType::Info, format!("Byte 4 (PRG Banks): {}", rom_data[4]));
    struct_terminal::print_logs(LogType::Info, format!("Byte 5 (CHR Banks): {}", rom_data[5]));
    struct_terminal::print_logs(LogType::Info, format!("Byte 6 (Flags 6)  : {:08b}", rom_data[6]));
    struct_terminal::print_logs(LogType::Info, format!("Byte 7 (Flags 7)  : {:08b}", rom_data[7]));
    struct_terminal::print_logs(LogType::Info, format!("Mapper ID -> {}", mapper_match));
    struct_terminal::print_logs(LogType::Info, format!("Has Trainer -> {}", has_trainer));
    
    // the size of the PRG ROM may be 16kb or 32kb, 
    //that info is at byte 4 as [1 if 16kb and 2 if 32kb]
    let program_size = rom_data[4] as usize * 0x4000; 

    let chr_size = rom_data[5] as usize * 0x2000; // then I take the chr size, which is at byte 5

    let has_persistent_storage = if (rom_data[6] & 0x02) != 0 { true } else { false };
    let mapper_save_path = if has_persistent_storage {
        Some(path)
    } else {
        None
    };

    let prg_rom_start = 16 + if has_trainer {512} else {0}; //the header size is 16 bytes
    let prg_rom_end = prg_rom_start + program_size; 
    let prg_rom_data = rom_data[prg_rom_start..prg_rom_end].into(); // mapping the actual game
    
    //The CHR ROM starts after the PRG ROM
    let chr_rom_data = rom_data[prg_rom_end..(prg_rom_end + chr_size)].into();
    
    let mirroring_type: Mirroring;

    if (rom_data[6] & 0x08) != 0 {
        todo!("FOUR SCREEN BIT")
    } else if (rom_data[6] & 0x01) != 0 {
        mirroring_type = Mirroring::Vertical;
    } else {
        mirroring_type = Mirroring::Horizontal;
    }

    match mapper_match {
        0 =>    Ok(wrap_in_pointers(mappers::InesMapper000::new(prg_rom_data, chr_rom_data, mirroring_type))),
        1 =>    Ok(wrap_in_pointers(mappers::InesMapper001::new(prg_rom_data, chr_rom_data, GameSave::new(path)))),
        2 =>    Ok(wrap_in_pointers(mappers::InesMapper002::new(prg_rom_data, mirroring_type, GameSave::new(path)))),
        3 =>    Ok(wrap_in_pointers(mappers::InesMapper003::new(prg_rom_data, chr_rom_data, mirroring_type, mapper_save_path))),
        4 =>    Ok(wrap_in_pointers(mappers::InesMapper004::new(prg_rom_data, chr_rom_data, mirroring_type, GameSave::new(path)))),
        9 =>    Ok(wrap_in_pointers(mappers::InesMapper009::new(prg_rom_data, chr_rom_data, mirroring_type, mapper_save_path))),
        10 =>   Ok(wrap_in_pointers(mappers::InesMapper010::new(prg_rom_data, chr_rom_data, mirroring_type, mapper_save_path))),
        163 =>  Ok(wrap_in_pointers(mappers::InesMapper163::new(prg_rom_data, chr_rom_data, mirroring_type, GameSave::new(path)))),

        _ => Err(format!("Mapper {} is not supported yet", mapper_match).into())
    }
}

