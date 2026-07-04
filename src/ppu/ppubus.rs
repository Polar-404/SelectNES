use crate::memory::mapper_base::*;

use std::rc::Rc; // Importe Rc
use std::cell::RefCell;

pub struct PPUBUS {
    //32 byte pallete [16 for backgroudn 16 for foreground]
    pub palette_ram: [u8; 0x20],
    vram: [u8; 0x0800], 
    pub mapper: Rc<RefCell<dyn Mapper>>, // 2KB VRAM
}
impl PPUBUS {
    pub fn new(mapper: Rc<RefCell<dyn Mapper>>) -> PPUBUS {
        PPUBUS {
            palette_ram: [0; 0x20],
            vram: [0; 0x0800],
            mapper,
        }
    }

    pub fn peek(&self, addr: u16) -> u8 {
        let addr =  addr & 0x3FFF;
        match addr {
            0x3F00..=0x3FFF => {
                let mut palette_addr = (addr & 0x1F) as usize;

                if palette_addr & 0x13 == 0x10 {
                    palette_addr &= 0x0F;
                }
                
                self.palette_ram[palette_addr]
                
            }
            0..=0x1FFF => {
                self.mapper.borrow().read_chr(addr)
            }
            0x2000..=0x3EFF  => {
                self.read_vram(addr)
            }
            _ => {
                todo!();
            }
        }     
    }

    pub fn write_ppubus(&mut self, addr: u16, data: u8) {
        let addr =  addr & 0x3FFF;
        match addr {

            0..=0x1FFF => {
                self.mapper.borrow_mut().write_chr(addr, data);
            }
            //VRAM (or nametable)
            0x2000..=0x3EFF  => {
                self.write_vram(addr, data);
            }
            0x3F00..=0x3FFF => {
                //mirroring the last addr of the palletes
                let mut palette_addr = (addr & 0x1F) as usize;

                if palette_addr >= 0x10 && palette_addr % 4 == 0 {
                    palette_addr -= 0x10;
                }
                //println!("data: {:#X}", data);
                self.palette_ram[palette_addr] = data;
            }
            _ => {
                unreachable!()
            }
        }
    }
    
    //TODO remover a referencia mutavel e limpar esse codigo depois que funcionar
    pub fn read_ppubus(&mut self, addr: u16) -> u8 {
        let addr =  addr & 0x3FFF;

        if addr < 0x3F00 {
            self.mapper.borrow_mut().notify_ppu_address(addr);
        }

        match addr {
            //minor optimzation ('fast-pathing' the palette since its the most common reading)
            0x3F00..=0x3FFF => {
                //mirroring the last addr of the palletes
                let mut palette_addr = (addr & 0x1F) as usize;

                if palette_addr & 0x13 == 0x10 {
                    palette_addr &= 0x0F;
                }
                
                self.palette_ram[palette_addr]
                
            }
            0..=0x1FFF => {
                self.mapper.borrow().read_chr(addr)
            }
            //VRAM (or nametable)
            0x2000..=0x3EFF  => {
                self.read_vram(addr)
            }
            _ => {
                todo!();
            }
        }
    }

    fn match_mirroring_addr(&self, addr: u16) -> usize {
        let addr = (addr - 0x2000) & 0x0FFF;
        match self.mapper.borrow().mirroring() {
            Mirroring::Vertical => (addr & 0x07FF) as usize,
            Mirroring::Horizontal => ((addr & 0x03FF) + ((addr & 0x0800) >> 1)) as usize,
            Mirroring::SingleScreenLower => (addr & 0x03FF) as usize,
            Mirroring::SingleScreenUpper => ((addr & 0x03FF) + 0x0400) as usize,
            //TODO didnt implement OneScreen mirroring yet
            #[allow(unreachable_patterns)]
            _ => {
                //with OneScreen mirroring all the nametables point to the same 1kb space 
                (addr & 0x03FF) as usize
            }
        }
    }

    fn write_vram(&mut self, addr: u16, data: u8) {
        let addr = self.match_mirroring_addr(addr);
        self.vram[addr] = data;
    }

    fn read_vram(&self, addr: u16) -> u8 {
        let addr = self.match_mirroring_addr(addr);
        self.vram[addr]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::mappers::dummy_mapper::TestMapper;

    fn make_bus_horizontal() -> PPUBUS {
        PPUBUS::new(TestMapper::new(vec![], Mirroring::Horizontal))
    }
    
    fn make_bus_vertical() -> PPUBUS {
        PPUBUS::new(TestMapper::new(vec![], Mirroring::Horizontal))
    }

    // ── Palette RAM ──────────────────────────────────────────────────────

    #[test]
    fn palette_escrita_e_leitura_basica() {
        let mut bus = make_bus_horizontal();
        bus.write_ppubus(0x3F05, 0x2C);
        assert_eq!(bus.read_ppubus(0x3F05), 0x2C);
    }

    #[test]
    fn palette_mirror_dentro_do_range_3f00_3fff() {
        let mut bus = make_bus_horizontal();
        // $3F25 deve espelhar $3F05 (0x25 & 0x1F = 0x05)
        bus.write_ppubus(0x3F05, 0xAA);
        assert_eq!(bus.read_ppubus(0x3F25), 0xAA);
    }

    #[test]
    fn palette_mirror_sprite_para_background_em_multiplos_de_4() {
        let mut bus = make_bus_horizontal();
        // $3F10, $3F14, $3F18, $3F1C espelham $3F00, $3F04, $3F08, $3F0C
        bus.write_ppubus(0x3F10, 0x11);
        assert_eq!(bus.read_ppubus(0x3F00), 0x11);

        bus.write_ppubus(0x3F14, 0x22);
        assert_eq!(bus.read_ppubus(0x3F04), 0x22);
    }

    #[test]
    fn palette_slots_independentes() {
        let mut bus = make_bus_horizontal();
        for i in 0u8..32 {
            // pula os que espelham ($10, $14, $18, $1C)
            if i >= 0x10 && i % 4 == 0 { continue; }
            bus.write_ppubus(0x3F00 + i as u16, i);
        }
        for i in 0u8..32 {
            if i >= 0x10 && i % 4 == 0 { continue; }
            assert_eq!(bus.read_ppubus(0x3F00 + i as u16), i);
        }
    }

    // ── VRAM — Mirroring horizontal ──────────────────────────────────────
    // A ($2000) = B ($2400)  →  VRAM baixo
    // C ($2800) = D ($2C00)  →  VRAM alto

    #[test]
    fn vram_horizontal_a_espelha_b() {
        let mut bus = make_bus_horizontal();
        bus.write_ppubus(0x2000, 0x42);
        assert_eq!(bus.read_ppubus(0x2400), 0x42);
    }

    #[test]
    fn vram_horizontal_b_espelha_a() {
        let mut bus = make_bus_horizontal();
        bus.write_ppubus(0x2400, 0x99);
        assert_eq!(bus.read_ppubus(0x2000), 0x99);
    }

    #[test]
    fn vram_horizontal_c_espelha_d() {
        let mut bus = make_bus_horizontal();
        bus.write_ppubus(0x2800, 0x77);
        assert_eq!(bus.read_ppubus(0x2C00), 0x77);
    }

    #[test]
    fn vram_horizontal_ab_independente_de_cd() {
        let mut bus = make_bus_horizontal();
        bus.write_ppubus(0x2000, 0x11);
        bus.write_ppubus(0x2800, 0x22);
        assert_eq!(bus.read_ppubus(0x2000), 0x11);
        assert_eq!(bus.read_ppubus(0x2800), 0x22);
    }

    // ── VRAM — mirror de $3000–$3EFF para $2000–$2EFF ───────────────────

    #[test]
    fn vram_mirror_de_3000_para_2000() {
        let mut bus = make_bus_horizontal();
        bus.write_ppubus(0x2005, 0xAB);
        assert_eq!(bus.read_ppubus(0x3005), 0xAB);
    }
    #[test]
    fn vertical_palette_escrita_e_leitura_basica() {
        let mut bus = make_bus_horizontal();
        bus.write_ppubus(0x3F05, 0x2C);
        assert_eq!(bus.read_ppubus(0x3F05), 0x2C);
    }

    #[test]
    fn vertical_palette_mirror_dentro_do_range_3f00_3fff() {
        let mut bus = make_bus_vertical();
        // $3F25 deve espelhar $3F05 (0x25 & 0x1F = 0x05)
        bus.write_ppubus(0x3F05, 0xAA);
        assert_eq!(bus.read_ppubus(0x3F25), 0xAA);
    }

    #[test]
    fn vertical_palette_mirror_sprite_para_background_em_multiplos_de_4() {
        let mut bus = make_bus_vertical();
        // $3F10, $3F14, $3F18, $3F1C espelham $3F00, $3F04, $3F08, $3F0C
        bus.write_ppubus(0x3F10, 0x11);
        assert_eq!(bus.read_ppubus(0x3F00), 0x11);

        bus.write_ppubus(0x3F14, 0x22);
        assert_eq!(bus.read_ppubus(0x3F04), 0x22);
    }

    #[test]
    fn vertica_palette_slots_independentes() {
        let mut bus = make_bus_vertical();
        for i in 0u8..32 {
            // pula os que espelham ($10, $14, $18, $1C)
            if i >= 0x10 && i % 4 == 0 { continue; }
            bus.write_ppubus(0x3F00 + i as u16, i);
        }
        for i in 0u8..32 {
            if i >= 0x10 && i % 4 == 0 { continue; }
            assert_eq!(bus.read_ppubus(0x3F00 + i as u16), i);
        }
    }

    // ── VRAM — Mirroring vertical ──────────────────────────────────────
    // A ($2000) = B ($2400)  →  VRAM baixo
    // C ($2800) = D ($2C00)  →  VRAM alto

    #[test]
    fn vram_vertical_a_espelha_b() {
        let mut bus = make_bus_vertical();
        bus.write_ppubus(0x2000, 0x42);
        assert_eq!(bus.read_ppubus(0x2400), 0x42);
    }

    #[test]
    fn vram_vertical_b_espelha_a() {
        let mut bus = make_bus_vertical();
        bus.write_ppubus(0x2400, 0x99);
        assert_eq!(bus.read_ppubus(0x2000), 0x99);
    }

    #[test]
    fn vram_vertical_c_espelha_d() {
        let mut bus = make_bus_vertical();
        bus.write_ppubus(0x2800, 0x77);
        assert_eq!(bus.read_ppubus(0x2C00), 0x77);
    }

    #[test]
    fn vram_vertical_ab_independente_de_cd() {
        let mut bus = make_bus_vertical();
        bus.write_ppubus(0x2000, 0x11);
        bus.write_ppubus(0x2800, 0x22);
        assert_eq!(bus.read_ppubus(0x2000), 0x11);
        assert_eq!(bus.read_ppubus(0x2800), 0x22);
    }

    // ── VRAM — mirror de $3000–$3EFF para $2000–$2EFF ───────────────────

    #[test]
    fn vertical_vram_mirror_de_3000_para_2000() {
        let mut bus = make_bus_vertical();
        bus.write_ppubus(0x2005, 0xAB);
        assert_eq!(bus.read_ppubus(0x3005), 0xAB);
    }
}
