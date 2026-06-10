// The main (and, as far as I know, only) difference between the Ricoh 2A03 used in the NES and the MOS 6502 is that the 20A4 has integrated audio.
// Since I haven't implemented audio, this effectively becomes a general-purpose 6502 microchip emulator. (except by the BUS, and NMI)

use serde::Deserialize;

use crate::cpu::opcodes::{self, opcodes_map};
use crate::engine::console::{LogType, print_logs};
use crate::memory::bus::BUS; 
use crate::memory::mapper_base::*;

use std::{collections::HashMap};

use std::rc::Rc;
use std::cell::RefCell;

bitflags! {

    /// This struct defines the CPU flags, and the `bitflags!` macro makes them easier to work with.
    #[derive(Debug, Clone, Copy)]
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const BREAK2            = 0b00100000; // isn't used on NES
        const OVERFLOW          = 0b01000000;
        const NEGATIVE          = 0b10000000;
    }
}

/// The stack on the 6502 is a "hard-coded" memory address that extends from **0x0100** to **0x01FF**.
const STACK: u16 = 0x0100;

/// While the stack extends to 0x01FF, **0xFD** is a common initial value for the stack pointer.
/// 
/// This starting point can help mitigate certain bugs.
const STACK_RESET: u8 = 0xfd;

/// The main (and, as far as I know, only) difference between the Ricoh 2A03 used in the NES and the MOS 6502 is that the 2A03 has integrated audio.
/// Since I haven't implemented audio, this effectively becomes a general-purpose 6502 microchip emulator.
pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,

    /// **Instructions that save or restore the flags map them to bits in the architectural 'P' register as follows:**
    ///
    ///```text 
    ///7  bit  0
    ///---- ----
    ///NV1B DIZC
    ///|||| ||||
    ///|||| |||+- Carry
    ///|||| ||+-- Zero
    ///|||| |+--- Interrupt Disable
    ///|||| +---- Decimal
    ///|||+------ (No CPU effect; see: the B flag)
    ///||+------- (No CPU effect; always pushed as 1)
    ///|+-------- Overflow
    ///+--------- Negative
    ///```
    pub status: CpuFlags,
    pub program_counter: u16,
    pub stack_pointer: u8, // SP
    pub cycles: u64,
    pub bus: BUS,
    
    pub vblank: bool,

    pub last_opcode: u8,
    is_halted: bool,
    nmi_prev: bool,
}
#[derive(Debug, Deserialize)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
 
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,

    Absolute,
    Absolute_X,
    Absolute_Y,

    Indirect_X,
    Indirect_Y,

    NoneAddressing,
}

impl CPU {
    pub fn new(mapper: Rc<RefCell<dyn Mapper>>) -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: CpuFlags::from_bits_truncate(0b100100),
            program_counter: 0,
            stack_pointer: STACK_RESET,
            bus: BUS::new(mapper),
            vblank: false,
            cycles: 0,
            last_opcode : 0,
            is_halted: false,
            nmi_prev: false,
        }
    }

    fn get_oprand_adress(&mut self, mode: &AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate => (self.program_counter, false), //pega o proximo imediato proximo valor
            //e joga na memoria (no register A)

            AddressingMode::ZeroPage => (self.bus.mem_read(self.program_counter) as u16, false), //

            AddressingMode::Absolute => (self.bus.mem_read_u16(self.program_counter), false),

            AddressingMode::ZeroPage_X => {
                let pos = self.bus.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                (addr, false)
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.bus.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                (addr, false)
            }
            AddressingMode::Absolute_X => {
                let base = self.bus.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                (addr, page_crossed)
            }
            AddressingMode::Absolute_Y => {
                let base = self.bus.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
                (addr, page_crossed)
            }
            AddressingMode::Indirect_X => {
                let base = self.bus.mem_read(self.program_counter);

                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.bus.mem_read(ptr as u16);
                let hi = self.bus.mem_read(ptr.wrapping_add(1) as u16);
                ((hi as u16) << 8 | (lo as u16), false)
            }
            AddressingMode::Indirect_Y => {
                let base = self.bus.mem_read(self.program_counter);

                let lo = self.bus.mem_read(base as u16);
                let hi = self.bus.mem_read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                let page_crossed = (deref_base & 0xFF00) != (deref & 0xFF00);
                (deref, page_crossed)
            }
            AddressingMode::NoneAddressing => {
                panic!("Panic at PC: {:04X}, Opcode: {:02X}. Tentou buscar endereço em NoneAddressing.", self.program_counter - 1, self.last_opcode);
            }

        }
    }

    pub fn trigger_cpu_nmi(&mut self) {
        //sends the vblank interuption to the cpu
        self.stack_push_u16(self.program_counter);

        let mut flags_to_push = self.status.clone();
        
        flags_to_push.insert(CpuFlags::BREAK2);
        flags_to_push.remove(CpuFlags::BREAK);
        
        self.stack_push(flags_to_push.bits());

        self.status.insert(CpuFlags::INTERRUPT_DISABLE);

        self.program_counter = self.bus.mem_read_u16(0xFFFA);

        self.cycles += 7;
        self.bus.tick(7);

        self.vblank = false;

    }

    pub fn trigger_cpu_irq(&mut self) {
        if self.status.contains(CpuFlags::INTERRUPT_DISABLE) {
            return;
        }

        self.stack_push_u16(self.program_counter);

        let mut status = self.status.bits();
        status &= !0x10;
        status |=  0x20;

        self.stack_push(status);

        self.status.insert(CpuFlags::INTERRUPT_DISABLE);

        let lo = self.bus.mem_read(0xFFFE);
        let hi = self.bus.mem_read(0xFFFF);
        self.program_counter = ((hi as u16) << 8) | lo as u16;

        //usually 7 on real hardware
        self.cycles += 7;
        self.bus.tick(7);
    }

    /////writes the program counter to
    //pub fn load(&mut self, program: Vec<u8>) {
    //    self.bus.load(program)
    //}

    ///resets the registers and flags to it's intial values, as well as the program counter to 0xFFFC
    pub fn reset_interrupt(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.status = CpuFlags::from_bits_truncate(0b100100);

        self.program_counter = self.bus.mem_read_u16(0xFFFC);
    }
    #[inline]
    fn update_zero_and_negative_flags(&mut self, result:u8) {
        self.status.set(CpuFlags::ZERO, result == 0);
        self.status.set(CpuFlags::NEGATIVE, result & 0b1000_0000 != 0);
    }


    // -------------processing bits and other logical cpu functions--------------

    ///LDA - Load A
    ///Takes a value at the given location (acording to the addressing mode) then loads it into the accumulator (register A)
    /// 
    /// (page crosses)
    fn lda(&mut self, mode: &AddressingMode) -> u8 {
        let (addr , page_crossed)= self.get_oprand_adress(mode);

        let value = self.bus.mem_read(addr);
        //println!("Lendo valor {:x} do endereço {:x}", value, addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);

        if page_crossed { 1 } else { 0 }
    }
    ///LDA - Load X
    /// 
    /// (page crosses)
    fn ldx(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let val = self.bus.mem_read(addr);

        self.register_x = val;
        self.update_zero_and_negative_flags(self.register_x);

        if page_crossed { 1 } else { 0 }
    }
    ///LDA - Load Y
    /// 
    /// (page crosses)
    fn ldy(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let val = self.bus.mem_read(addr);

        self.register_y = val;
        self.update_zero_and_negative_flags(self.register_y);

        if page_crossed { 1 } else { 0 }
    }
    ///TXA e TYA
    fn transfer_to_accumulator(&mut self, register: u8) {
        self.register_a = register;
        self.update_zero_and_negative_flags(self.register_a);
    }

    ///STA - Store A
    ///stores the accumulator into the target memory addrss
    /// 
    /// (page crosses)
    fn sta(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        self.bus.mem_write(addr, self.register_a); //o contrario do LDA, ainda usando os mesmos parametros do LDA
        //mas esse escreve o que esta no register na memoria

        0
    }

    ///Rotate Left
    /// 
    /// (page crosses)
    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        let mut data = self.bus.mem_read(addr);
        let previous_carry = self.status.contains(CpuFlags::CARRY);

        if data & 0b1000_0000 != 0 {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        data = data << 1;
        if previous_carry {
            data = data | 0b0000_0001
        }
        self.bus.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);

        0
    }
    ///Rotate Left Register A
    fn rol_accumulator(&mut self) {
        let mut data = self.register_a;
        let previous_carry = self.status.contains(CpuFlags::CARRY);

        if data & 0b1000_0000 != 0 {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        data = data << 1;
        if previous_carry {
            data = data | 0b0000_0001
        }
        self.register_a = data;
        self.update_zero_and_negative_flags(self.register_a);
    }

    ///ROR - Rotate Right
    /// 
    /// (page crosses)
    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        let mut data = self.bus.mem_read(addr);
        let previous_carry = self.status.contains(CpuFlags::CARRY);

        if data & 0b0000_0001 != 0 {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        data = data >> 1;
        if previous_carry {
            data = data | 0b1000_0000
        }
        self.bus.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);

        0
    }
    ///Rotate Right Register A
    fn ror_accumulator(&mut self) {
        let mut data = self.register_a;
        let previous_carry = self.status.contains(CpuFlags::CARRY);

        if data & 0b0000_0001 != 0 {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        data = data >> 1;
        if previous_carry {
            data = data | 0b1000_0000
        }
        self.register_a = data;
        self.update_zero_and_negative_flags(self.register_a);
    }

    ///Bitwise AND
    /// 
    /// (page crosses)
    fn and(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let data = self.bus.mem_read(addr);
        self.register_a = self.register_a & data;
        self.update_zero_and_negative_flags(self.register_a);

        if page_crossed { 1 } else { 0 }
    }
    ///BIT - Bit Test
    /// 
    /// (page crosses)
    fn bit(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        let data = self.bus.mem_read(addr);
        let modification = self.register_a & data;

        self.status.set(CpuFlags::ZERO, modification == 0);
        self.status.set(CpuFlags::NEGATIVE, data & 0b1000_0000 != 0);
        self.status.set(CpuFlags::OVERFLOW, data & 0b0100_0000 != 0);

        0
    }

    ///EOR - Bitwise Exclusive OR
    /// 
    /// (page crosses)
    fn eor(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let data = self.bus.mem_read(addr);
        self.register_a = self.register_a ^ data;
        self.update_zero_and_negative_flags(self.register_a);

        if page_crossed { 1 } else { 0 }
    }
    ///ORA - Bitwise OR
    /// 
    /// (page crosses)
    fn ora(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let data = self.bus.mem_read(addr);
        self.register_a = self.register_a | data;
        self.update_zero_and_negative_flags(self.register_a);

        if page_crossed { 1 } else { 0 }
    }

    ///Decrement X
    #[inline]
    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }
    ///Decrement Y
    #[inline]
    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }
    ///DEC
    /// 
    /// (page crosses)
    fn dec_mem(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        let data = self.bus.mem_read(addr);
        
        let modification = data.wrapping_sub(1);
        self.bus.mem_write(addr, modification);
        self.update_zero_and_negative_flags(modification);

        0
    }

    /// ASL - Arithmetic Shift Left
    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        let mut data = self.bus.mem_read(addr);
        if data >> 7 == 1 {
            self.status.insert(CpuFlags::CARRY);
        }
        else {
            self.status.remove(CpuFlags::CARRY);
        }
        data = data << 1;
        self.bus.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        return data
    }
    /// ASL - Arithmetic Shift Left
    fn asl_accumulator(&mut self) -> u8 {
        if (self.register_a & 0b1000_0000) != 0 {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        self.register_a = self.register_a << 1;
        self.update_zero_and_negative_flags(self.register_a);
        self.register_a
    }
    ///LSR - Logical Shift Right
    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        let mut data = self.bus.mem_read(addr);
        if (data & 0b0000_0001) != 0 {
            self.status.insert(CpuFlags::CARRY);
        }
        else {
            self.status.remove(CpuFlags::CARRY);
        }
        data = data >> 1;
        self.bus.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        return data
    }
    ///LSR - Logical Shift Right
    fn lsr_accumulator(&mut self) -> u8 {
        if (self.register_a & 0b0000_0001) != 0 {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        self.register_a = self.register_a >> 1;
        self.update_zero_and_negative_flags(self.register_a);
        self.register_a
    }
    ///JMP - Jump
    fn jmp_abs(&mut self) {
        let location = self.bus.mem_read_u16(self.program_counter);
        self.program_counter = location;
    }
    ///JMP - Jump
    fn jmp_indrect(&mut self) {
        let location = self.bus.mem_read_u16(self.program_counter);
        let indirect_ref: u16;
        if location & 0x00FF == 0x00FF {
            // implementação de um bug do 6502 que não rotaciona corretamente os bytes
            // quando o low byte era terminado em FF ele n passava o hi byte para o proximo, 
            // ao inves disso ele simplesmente dava "wrapping" no low byte
            let lo = self.bus.mem_read(location);
            let hi = self.bus.mem_read(location & 0xFF00);
            indirect_ref = (hi as u16) << 8 | (lo as u16);
        } else {
            indirect_ref = self.bus.mem_read_u16(location)
        }
        self.program_counter = indirect_ref;
    }

    ///SEC - Set Carry
    #[inline]
    fn sec(&mut self) {
        self.status.insert(CpuFlags::CARRY);
    }
    ///limpa o CARRY flag
    #[inline]
    fn clc(&mut self) {
        self.status.remove(CpuFlags::CARRY)
    }    
    ///SED - Set Decimal
    #[inline]
    fn sed(&mut self) {
        self.status.insert(CpuFlags::DECIMAL_MODE);
    }
    ///limpa o DECIMAL_MODE flag
    #[inline]
    fn cld(&mut self) {
        self.status.remove(CpuFlags::DECIMAL_MODE);
    }

    ///SEI - Set Interrupt Disable
    #[inline]
    fn sei(&mut self) {
        self.status.insert(CpuFlags::INTERRUPT_DISABLE);
    }

    ///limpa o INTERRUPT_DISABLE flag
    #[inline]
    fn cli(&mut self) {
        self.status.remove(CpuFlags::INTERRUPT_DISABLE);
    }
    ///limpa o OVERFLOW flag
    #[inline]
    fn clv(&mut self) {
        self.status.remove(CpuFlags::OVERFLOW);
    }

    /// ---------------- COMANDOS DA STACK ------------ //
    #[inline]
    fn increment_stack(&mut self) {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
    }
    fn stack_pop(&mut self) -> u8 {
        //if self.stack_pointer == 0xfd {
        //    panic!("tried to pop a empty stack")
        //}
        self.increment_stack();
        //println!("addr: {}", STACK + self.stack_pointer as u16);
        let data = self.bus.mem_read( STACK + self.stack_pointer as u16) ;
        //println!("data {}",data);
        data
    }
    ///PULL ACCUMULATOR
    fn pla(&mut self) {
        self.register_a = self.stack_pop();
        self.update_zero_and_negative_flags(self.register_a);
    }
    /// PUSH ACCUMULATOR
    fn pha(&mut self, data: u8) {
        self.bus.mem_write(STACK + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }
    ///Pull Processor Status
    fn plp(&mut self) {
//TODO: necessario revisão e testes
        let flags = CpuFlags::from_bits_truncate(self.stack_pop());
        self.status = flags;
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK2);
    }
    fn stack_push(&mut self, data: u8) {
        self.bus.mem_write((STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }
    ///Push Processor Status
    fn php(&mut self) {
        self.stack_push(self.status.bits() | 0b0011_0000);
    }
    ///TSX - Transfer Stack Pointer to X
    fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
        self.update_zero_and_negative_flags(self.register_x);
    }
    ///TXS - Transfer X to Stack Pointer
    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
        //self.update_zero_and_negative_flags(self.stack_pointer);
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }
    //Jump to Subroutine
    fn jsr(&mut self) {
        self.stack_push_u16(self.program_counter.wrapping_add(1));
        self.program_counter = self.bus.mem_read_u16(self.program_counter)
        
    }
    fn write_register(&mut self, mode: &AddressingMode, register: u8) {
        let (addr, _) = self.get_oprand_adress(mode);
        self.bus.mem_write(addr, register);
    }
    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        (hi << 8) | lo
    }

    ///RTI - Return from Interrupt
    fn rti(&mut self) {
        self.status = CpuFlags::from_bits_truncate(self.stack_pop());
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK2);
        self.program_counter = self.stack_pop_u16();
    }
    ///RTS - Return from Subroutine
    fn rts(&mut self) {
        self.program_counter = self.stack_pop_u16();
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    // ------------------ CONDITIONS ------------------ //


    fn branch_if(&mut self, condition: bool) -> u8 {
        if condition {
            let mut extra_cycles = 1;

            let offset = self.bus.mem_read(self.program_counter) as i8;
            let base_addr = self.program_counter.wrapping_add(1);
            let new_program_counter = base_addr.wrapping_add(offset as u16);

            if (base_addr & 0xFF00) != (new_program_counter & 0xFF00) {
                extra_cycles += 1;
            }

            self.program_counter = new_program_counter;
            extra_cycles
        } else {
            0
        }
    }
    ///CMP 
    /// 
    /// (page crosses)
    fn compare(&mut self, mode: &AddressingMode, register: u8) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let mem_val = self.bus.mem_read(addr);

        if register >= mem_val {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        let result = register.wrapping_sub(mem_val);

        if result == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        if (result & 0b1000_0000) != 0 {
            self.status.insert(CpuFlags::NEGATIVE);
        } else {
            self.status.remove(CpuFlags::NEGATIVE);
        }

        if page_crossed { 1 } else { 0 }
    }

    ///ADC - Add with Carry
    /// 
    /// soma dois numeros e adiciona um bit de carry caso aconteça overflow
    /// SOMA OS NUMEROS DO REGISTRADOR A + O VALOR NO ENDEREÇO DE MEMORIA PASSADO
    /// depois finaliza o comando passando o resultado para o registrador A e dando update nas flags ZERO e NEGATIVE
    /// 
    /// (page crosses)
    fn adc(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let val = self.bus.mem_read(addr);

        let sum = self.register_a as u16 + val as u16
        + (if self.status.contains(CpuFlags::CARRY) {
            1
        } else {
            0
        });

        if sum > 0xff {
            self.status.insert(CpuFlags::CARRY); 
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        
        let result = sum as u8;

        if (result ^ val) & (result ^ self.register_a) & 0b1000_0000 != 0 {
            self.status.insert(CpuFlags::OVERFLOW);
        } else {
            self.status.remove(CpuFlags::OVERFLOW);
        }
        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);

        if page_crossed { 1 } else { 0 }
    }

    ///SBC - Subtract with Carry
    /// 
    /// (page crosses)
    fn sbc(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let val = self.bus.mem_read(addr);

        let val_complement = !val;
        let carry = if self.status.contains(CpuFlags::CARRY) { 1u16 } else { 0u16 };
        let sum = self.register_a as u16 + val_complement as u16 + carry;

        if sum > 0xff { //seta a carry flag
            self.status.insert(CpuFlags::CARRY); 
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        
        let result = sum as u8;

        if ((self.register_a ^ val) & (self.register_a ^ result) & 0b1000_0000) != 0 { //seta a overflow flag
            //usa os operadores logicos de XOR para verificar quais bits sao diferentes e dps verifica com 0b100...
            //já q é o bit que quer ser verificado(o unico bit q importa)... se for diferente dos outros significa que
            //ocorreu um overflow, 
            self.status.insert(CpuFlags::OVERFLOW);
        } else {
            self.status.remove(CpuFlags::OVERFLOW);
        }
        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);

        if page_crossed { 1 } else { 0 }
    }

    ///TAX - Transfer A to X
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }
    ///TAX - Transfer A to Y
    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_flags(self.register_y);
    }

    ///INC - Increment Memory
    ///
    ///(page crosses)
    fn inc_mem(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        let data = self.bus.mem_read(addr);
        
        let modification = data.wrapping_add(1);
        self.bus.mem_write(addr, modification);
        self.update_zero_and_negative_flags(modification);

        0
    }
    ///INX - Increment X
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }
    ///INY - Increment Y
    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    /// BRK - Force Interrupt
    fn brk(&mut self) {
        let return_addr = self.program_counter.wrapping_add(1);
        self.stack_push_u16(return_addr);

        let mut flags = self.status.clone();
        flags.insert(CpuFlags::BREAK);
        flags.insert(CpuFlags::BREAK2);
        self.stack_push(flags.bits());

        self.status.insert(CpuFlags::INTERRUPT_DISABLE);

        self.program_counter = self.bus.mem_read_u16(0xFFFE);
    }

    // -------------- undocumented / unofficial --------------

    ///(page crosses)
    fn lax(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode); 
        let value = self.bus.mem_read(addr);

        self.register_a = value;
        self.register_x = value;

        self.update_zero_and_negative_flags(value);

        if page_crossed {
            1
        } else {
            0
        }
    }

    fn axs(&mut self) {
        let and_result = self.register_x & self.register_a;
        let immediate = self.bus.mem_read(self.program_counter);

        self.program_counter = self.program_counter.wrapping_add(1);

        if and_result >= immediate {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        let final_result = and_result.wrapping_sub(immediate);
        self.register_x = final_result;

        self.update_zero_and_negative_flags(self.register_x);
    }

    fn slo(&mut self, mode: &AddressingMode) -> u8 {
        //ASL
        let (addr, page_crossed) = self.get_oprand_adress(mode);
        let mut data = self.bus.mem_read(addr);
        if data >> 7 == 1 {
            self.status.insert(CpuFlags::CARRY);
        }
        else {
            self.status.remove(CpuFlags::CARRY);
        }
        data = data << 1;
        self.bus.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        
        //ORA
        self.register_a = self.register_a | data;
        self.update_zero_and_negative_flags(self.register_a);

        if page_crossed { 1 } else { 0 }
    }

    fn rra(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, _) = self.get_oprand_adress(mode);
        let mut val = self.bus.mem_read(addr);

        let carry_in = if self.status.contains(CpuFlags::CARRY) { 1 } else { 0 };
        let carry_out = val & 1;
        
        val = (val >> 1) | (carry_in << 7);
        
        if carry_out == 1 {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        self.bus.mem_write(addr, val);

        let sum = self.register_a as u16 + val as u16 + carry_out as u16;
        let result = sum as u8;

        if (self.register_a ^ result) & (val ^ result) & 0b1000_0000 != 0 {
            self.status.insert(CpuFlags::OVERFLOW);
        } else {
            self.status.remove(CpuFlags::OVERFLOW);
        }

        if sum > 0xFF {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);

        0 
    }

    fn rla(&mut self, mode: &AddressingMode) -> u8 {
        let (addr, page_crossed) = self.get_oprand_adress(mode);

        let mut data = self.bus.mem_read(addr);
        let previous_carry = self.status.contains(CpuFlags::CARRY);

        if data & 0b1000_0000 != 0 {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        
        data = data << 1;
        if previous_carry {
            data = data | 0b0000_0001
        }

        self.bus.mem_write(addr, data);

        self.register_a = self.register_a & data;
        self.update_zero_and_negative_flags(self.register_a);

        if page_crossed { 1 } else { 0 }
    }

    pub fn step(&mut self) -> (bool, u8)  {
        self.step_with_callback(None::<fn(&mut Self)>)
    }

    pub fn step_with_callback<F>(&mut self, mut callback: Option<F>) -> (bool, u8) 
    where F: FnMut(&mut CPU) {
        let ref opcodes: &'static HashMap<u8, &'static opcodes::OpCode> = opcodes_map();

        if let Some(ref mut c) = callback {
            c(self)
        }

        let nmi_current = self.bus.nmi_active();

        if nmi_current && !self.nmi_prev {
            print_logs(LogType::Debug, format!("Vblank Triggered [PC:{} | A:{} | X:{} | Y:{} CYC: {}]",
                self.bus.peek(self.program_counter), self.register_a, self.register_x, self.register_y, self.cycles
            ));
            self.trigger_cpu_nmi();
        }
        
        self.nmi_prev = nmi_current;

        if self.bus.mapper.borrow_mut().irq_pending() && !self.status.contains(CpuFlags::INTERRUPT_DISABLE) {
            self.trigger_cpu_irq();
        }

        self.last_opcode = self.bus.mem_read(self.program_counter);
        self.program_counter += 1;

        let program_counter_state = self.program_counter;

        let opcode = match opcodes.get(&self.last_opcode) {
            Some(op) => op,
            None => {
                print_logs(LogType::Warning, format!("Ilegal Opcode {:02X}, at PC:{:04X}, CPU is halted",
                    self.last_opcode, self.program_counter - 1
                ));
                self.is_halted = true;
                return (true, 0);
            }
        };
        
        let mut opcycles = opcode.cycles;

        let mut is_2002_read = false;
        if opcode.len == 3 && (self.last_opcode == 0x2C || self.last_opcode == 0xAD) {
            let lo = self.bus.mem_read(self.program_counter);
            let hi = self.bus.mem_read(self.program_counter.wrapping_add(1));
            let addr = ((hi as u16) << 8) | (lo as u16);
            
            if addr == 0x2002 {
                is_2002_read = true;
            }
        }

        if is_2002_read {
            self.bus.tick(opcycles);
        }

        match self.last_opcode {
            //LDA
            0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                //cada uma das instruções representa o comando LDA mas como flags diferentes
                opcycles += self.lda(&opcode.mode);
            }

            //STA
            0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                opcycles += self.sta(&opcode.mode);
            }

            //ADC
            0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => {
                opcycles += self.adc(&opcode.mode);
            }
            //SBC
            0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => {
                opcycles += self.sbc(&opcode.mode);
            }

            //CMP (COMPARE A)
            0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => {
                opcycles += self.compare(&opcode.mode, self.register_a);
            }

            //CPX (COMPARE X)
            0xE0 | 0xE4 | 0xEC => {
                opcycles += self.compare(&opcode.mode, self.register_x);
            }

            //CPY (COMPARE Y)
            0xC0 | 0xC4 | 0xCC => {
                opcycles += self.compare(&opcode.mode, self.register_y);
            }

            //STX - Store X
            0x86 | 0x96 | 0x8E => {
                self.write_register(&opcode.mode, self.register_x);
            }
            //STY - Store Y
            0x84 | 0x94 | 0x8C => {
                self.write_register(&opcode.mode, self.register_y);
            }
            //AND BITWISE
            0x21 | 0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x31 => {
                opcycles += self.and(&opcode.mode);
            }

            //ASL - Arithmetic Shift Left
            0x06 | 0x16 | 0x0E | 0x1E => { 
                self.asl(&opcode.mode);
            }
            0x0A => {
                self.asl_accumulator();
            }
            //LSR - Logical Shift Right
            0x46 | 0x56 | 0x4e | 0x5e => {
                self.lsr(&opcode.mode);
            }
            0x4A => {
                self.lsr_accumulator();
            }
            // BIT - Bit Test
            0x24 | 0x2C => {
                opcycles += self.bit(&opcode.mode);
            }
            // ORA
            0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => {
                opcycles += self.ora(&opcode.mode);
            }
            // EOR - Exclusive OR
            0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => {
                opcycles += self.eor(&opcode.mode);
            }

            //DEC - Decrement Memory
            0xC6 | 0xD6 | 0xCE | 0xDE => {
                opcycles += self.dec_mem(&opcode.mode);
            }
            //DEX - Decrement X
            0xCA => self.dex(),
            //DEY - Decrement Y
            0x88 => self.dey(),

            // BCC - Branch if Carry Clear
            0x90 => { opcycles += self.branch_if(!self.status.contains(CpuFlags::CARRY)); }
            // BCS - Branch if Carry Set
            0xB0 => { opcycles += self.branch_if(self.status.contains(CpuFlags::CARRY)); }
            // BEQ - Branch if Equal
            0xF0 => { opcycles += self.branch_if(self.status.contains(CpuFlags::ZERO)); }
            // BNE - Branch if Not Equal
            0xD0 => { opcycles += self.branch_if(!self.status.contains(CpuFlags::ZERO)); }
            // BPL - Branch if Plus
            0x10 => { opcycles += self.branch_if(!self.status.contains(CpuFlags::NEGATIVE)); }
            // BMI - Branch if Minus
            0x30 => { opcycles += self.branch_if(self.status.contains(CpuFlags::NEGATIVE)); }
            // BVC - Branch if Overflow Clear
            0x50 => { opcycles += self.branch_if(!self.status.contains(CpuFlags::OVERFLOW)); }
            // BVS - Branch if Overflow Set
            0x70 => { opcycles += self.branch_if(self.status.contains(CpuFlags::OVERFLOW)); }

            //SET FLAGS
            0x38 => self.sec(),
            0xF8 => self.sed(),
            0x78 => self.sei(),
            
            //CLEAR FLAGS
            0x18 => self.clc(),
            0xD8 => self.cld(),
            0x58 => self.cli(),
            0xB8 => self.clv(),

            //LDX 
            0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => {
                opcycles += self.ldx(&opcode.mode);
            }
            //LDY
            0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => {
                opcycles += self.ldy(&opcode.mode);
            }
            //TAX
            0xAA => self.tax(),
            //TAY
            0xA8 => self.tay(),

            //INC
            0xE6 | 0xF6 | 0xEE | 0xFE => {
                opcycles += self.inc_mem(&opcode.mode);
            }

            //ROL ACCUMULATOR / ROtate Accumulator Left
            0x2A => self.rol_accumulator(),

            //ROL - Rotate Left
            0x26 | 0x36 | 0x2E | 0x3E => {
                opcycles += self.rol(&opcode.mode);
            }

            //ROR ACCUMULATOR / Rotate Accumulator Right
            0x6A => self.ror_accumulator(),

            //ROR - Rotate Right
            0x66 | 0x76 | 0x6E | 0x7E => {
                opcycles += self.ror(&opcode.mode);
            }

            //PLA
            0x68 => self.pla(),
            //PHA
            0x48 => self.pha(self.register_a),
            //PLP
            0x28 => self.plp(),
            //PHP - Push Processor Status
            0x08 => self.php(),

            //RTI - Return from Interrupt
            0x40 => self.rti(),

            //RTS - Return from Subroutine
            0x60 => self.rts(),

            //TSX
            0xBA => self.tsx(),
            //TXS
            0x9A => self.txs(),

            //JSR
            0x20 => self.jsr(),

            //JMP
            0x4C => self.jmp_abs(),
            0x6C => self.jmp_indrect(),

            //INX
            0xe8 => self.inx(),

            //INY
            0xC8 => self.iny(),

            //TXA
            0x8A => self.transfer_to_accumulator(self.register_x),
            //TYA
            0x98 => self.transfer_to_accumulator(self.register_y),

            //NOP - No Operation
            0xEA | 0x04 | 0x0C | 0x14 | 0x1A | 0x1C | 0x34 | 0x3A |
            0x3C | 0x44 | 0x54 | 0x5A | 0x5C | 0x64 | 0x74 | 0x7A | 
            0x7C | 0x80 | 0x82 | 0x89 | 0xC2 | 0xD4 | 0xDA | 0xDC | 
            0xE2 | 0xF4 | 0xFA | 0xFC => { }

            //LAX
            0xA3 | 0xA7 | 0xAB | 0xAF | 0xB3 | 0xB7 | 0xBF => {
                opcycles += self.lax(&opcode.mode)
            }

            //AXS
            0xCB => self.axs(),

            //SLO
            0x03 | 0x07 | 0x0F | 0x13 | 0x17 | 0x1B | 0x1F => {
                opcycles += self.slo(&opcode.mode)
            }

            0x63 | 0x67 | 0x6F | 0x73 | 0x77 | 0x7B | 0x7F => {
                opcycles += self.rra(&opcode.mode)
            }

            0x3B | 0x23 | 0x27 | 0x2F | 0x33 | 0x37 | 0x3F => {
                opcycles += self.rla(&opcode.mode)
            }

            //BRK
            0x00 => self.brk(),

            _ => { 
                print_logs(LogType::Warning, format!(
                    "CPU INSTRUCTION NOT IMPLEMENTED! ( CODE:{:02X}, at PC:{:04X} ), 
                    CPU is halted to prevent undefined behavior",
                    self.last_opcode, self.program_counter - 1
                ));
                self.is_halted = true;
                return (true, 0);
            }
        }

        if !is_2002_read {
            self.bus.tick(opcycles); 
        } else {
            let extra_cycles = opcycles - opcode.cycles;
            if extra_cycles > 0 {
                self.bus.tick(extra_cycles);
            }
        }

        self.cycles += opcycles as u64;

        //increments the program counter accordingly to how many cicles the opcode is specified at the hashmap
        if program_counter_state == self.program_counter {
            self.program_counter += (opcode.len -1) as u16;
        }
        (false, opcycles)
    }

    pub fn format_cpu_status(status: u8) -> String {
        let mut s = String::new();
        s.push_str(if (status & 0b10000000) != 0 { "N" } else { "-" });
        s.push_str(if (status & 0b01000000) != 0 { "V" } else { "-" });
        s.push_str("-"); // Ignored bit
        s.push_str(if (status & 0b00010000) != 0 { "B" } else { "-" });
        s.push_str(if (status & 0b00001000) != 0 { "D" } else { "-" });
        s.push_str(if (status & 0b00000100) != 0 { "I" } else { "-" });
        s.push_str(if (status & 0b00000010) != 0 { "Z" } else { "-" });
        s.push_str(if (status & 0b00000001) != 0 { "C" } else { "-" });
        s
    }

}

///Simple tests module for the cpu
#[cfg(test)]
mod test {
    #[allow(unused)]
    use crate::memory::mapper_base::*;

    use crate::memory::mappers::dummy_mapper::TestMapper;
    use super::*;

    impl CPU {
        pub fn run_test(&mut self) {
            self.reset_interrupt();
            loop {
                let opcode = self.bus.mem_read(self.program_counter);
                
                if opcode == 0x00 { break; }

                self.step();
            }
        }
    }

    // ------------------- LDA ------------------
    #[test]
    fn test_0xa9_lda_immediate() {

        let mapper = TestMapper::new(vec![0xa9, 0x05, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();
        assert_eq!(cpu.register_a, 0x05);
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    }
    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mapper = TestMapper::new(vec![0xa9, 0x00, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();
        assert!(cpu.status.contains(CpuFlags::ZERO));
    }
  
    #[test]
    fn test_lda_from_memory() {
        let mapper = TestMapper::new(vec![0xA5, 0x10, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0x55);

        cpu.run_test();
        assert_eq!(cpu.register_a, 0x55) //0xA5 é o LDA zero page, procurando no endereço 
        //de memoria 0x10, e dps break
    }
    #[test]
    fn test_ldx_from_memory() {
        let mapper = TestMapper::new(vec![0xA6, 0x10, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0x55);

        cpu.run_test();
        assert_eq!(cpu.register_x, 0x55) //0xA5 é o LDA zero page, procurando no endereço 
        //de memoria 0x10, e dps break
    }

    #[test]
    fn test_ldy_from_memory() {
        let mapper = TestMapper::new(vec![0xA4, 0x10, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0x55);

        cpu.run_test();
        assert_eq!(cpu.register_y, 0x55) //0xA5 é o LDA zero page, procurando no endereço 
        //de memoria 0x10, e dps break
    }

    // ------------------- TAX ------------------
    #[test]
    fn test_0xaa_tax() {
        let mapper = TestMapper::new(vec![0xa9, 0x0a, 0xAA, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test(); //primeiro inserir LDA, no register A, o valor 0x0a(q é 10)
        //depois coloca esse valor no register x como comando TAX (0xAA), depois break
        assert_eq!(cpu.register_x, 10)
    }
    // ------------------- INX ------------------
    #[test]
    fn test_0xe8_inx() {
        let mapper = TestMapper::new(vec![0xE8, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();

        assert_eq!(cpu.register_x, 1)
    }
    // --------------- WRITE MEMORY --------------------
    #[test]
    fn test_write_mem() {
        let mapper = TestMapper::new(vec![], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write_u16(0x1fef, 0xef);

        assert_eq!(cpu.bus.mem_read_u16(0x1fef), 0xef);
    }

    #[test]
    // -------------------- ADC ------------------------
    fn test_adc_from_immediate() {
        let mapper = TestMapper::new(vec![0x69, 0x10, 0x69, 0x32], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        //cpu.bus.mem_write(0x69, data);
        cpu.run_test();
        assert_eq!(cpu.register_a, 0x42);
    }
    #[test]
    fn test_adc_from_memory() {
        let mapper = TestMapper::new(vec![
        0xa5, 0x10, // LDA ZeroPage, operando 0x10
        0x65, 0x20, // ADC ZeroPage, operando 0x20
        0x00], 
        Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0xab); //carrega 0xab
        cpu.bus.mem_write(0x20, 0x05); //carrega 0x02

        //le o 0xf5, dps le o 0x31
        cpu.run_test();      // BRK]);
        assert_eq!(cpu.register_a, 0xb0)
    }
    #[test]
    fn test_adc_carry_flag() {
        let mapper = TestMapper::new(vec![0xa5, 0x10, 0x65, 0x20, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0xff);
        cpu.bus.mem_write(0x20, 0x01);
        cpu.run_test();
        assert_eq!(cpu.register_a, 0x00);

        assert!(cpu.status.contains(CpuFlags::ZERO)); //zero flag foi setada
        assert!(cpu.status.contains(CpuFlags::CARRY)); //carry flag foi setada
    }
    #[test]
    fn test_adc_carry_sum() {
        let mapper = TestMapper::new(vec![0xa5, 0x10, 0x65, 0x20, 0x69, 0x50, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0xff);
        cpu.bus.mem_write(0x20, 0x01);
        cpu.run_test();

        assert!(!cpu.status.contains(CpuFlags::ZERO)); //zero flag foi desligada
        assert!(!cpu.status.contains(CpuFlags::CARRY)); //carry flag foi desligada

        assert_eq!(cpu.register_a, 0x51);
    }
    #[test]
    fn test_clear_flags() {
        //const CARRY             = 0b00000001;
        //const ZERO              = 0b00000010;
        //const INTERRUPT_DISABLE = 0b00000100;
        //const DECIMAL_MODE      = 0b00001000; 
        //const BREAK             = 0b00010000;
        //const BREAK2            = 0b00100000;
        //const OVERFLOW          = 0b01000000;
        //const NEGATIVE          = 0b10000000;

        let mapper = TestMapper::new(vec![], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.status = CpuFlags::from_bits_truncate(0b0100_1101);

        //respectivamente [clc, cld, cli, clv]
        assert!(cpu.status.contains(CpuFlags::CARRY));
        assert!(cpu.status.contains(CpuFlags::DECIMAL_MODE));
        assert!(cpu.status.contains(CpuFlags::INTERRUPT_DISABLE));
        assert!(cpu.status.contains(CpuFlags::OVERFLOW));

        cpu.clc(); cpu.cld(); cpu.cli(); cpu.clv();

        assert!(!cpu.status.contains(CpuFlags::CARRY));
        assert!(!cpu.status.contains(CpuFlags::DECIMAL_MODE));
        assert!(!cpu.status.contains(CpuFlags::INTERRUPT_DISABLE));
        assert!(!cpu.status.contains(CpuFlags::OVERFLOW));
    }

    #[test]
    fn test_set_flags() {
        let mapper = TestMapper::new(vec![0x38, 0xF8, 0x78], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();

        assert!(cpu.status.contains(CpuFlags::CARRY));
        assert!(cpu.status.contains(CpuFlags::DECIMAL_MODE));
        assert!(cpu.status.contains(CpuFlags::INTERRUPT_DISABLE));
    }
    #[test]
    fn test_compare() {
        let mapper = TestMapper::new(vec![0xa9, 0x2f, 0xC5, 0x10,], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0x2f);
        cpu.run_test();

        assert!(cpu.status.contains(CpuFlags::ZERO));
        assert!(cpu.status.contains(CpuFlags::CARRY));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    }
    #[test]
    fn test_and_instruction() {
        let mapper = TestMapper::new(vec![0xa9, 0x1A, 0x25, 0x10,], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);
        
        cpu.bus.mem_write(0x10, 0x5C); //0x5C == 0b0101_1100
        cpu.run_test(); //0x1A == 0b0001_1010
        // and == 0b0001_1000
        assert_eq!(cpu.register_a, 0x18);
    }
    #[test]
    fn test_asl_instruction() {
        let mapper = TestMapper::new(vec![0x06, 0x10], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0b0001_0000);
        cpu.run_test();
        assert_eq!(cpu.bus.mem_read(0x10), 0b0010_0000);
    }
    #[test]
    fn test_bcc_instruction() {
        let mapper = TestMapper::new(vec![0x90, 0x03, 0x00, 0x00, 0x00, 0xa9, 0xff, 0x00 ], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        // |0x38 - sec | 0x90 - bcc | 0x18 - clc |
        cpu.run_test();
        assert_eq!(cpu.register_a, 0xff);

        let mapper = TestMapper::new(vec![0x38, 0x90, 0x02,  0xa9, 0xab, 0x00,], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);
        cpu.run_test();
        assert_eq!(cpu.register_a, 0xab);
    }
    #[test]
    fn test_bit_instruction() {
        let mapper = TestMapper::new(vec![0xa9, 0xf8, 0x24, 0x20], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x20, 0xf3);
        cpu.run_test();
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(cpu.status.contains(CpuFlags::NEGATIVE));
        assert!(cpu.status.contains(CpuFlags::OVERFLOW));
    }
    #[test]
    fn test_decrement_memory() {
        let mapper = TestMapper::new(vec![0xc6, 0x50, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x50, 0x0f);
        cpu.run_test();
        assert_eq!(cpu.bus.mem_read(0x50), 0x0e);
    }
    #[test]
    fn test_decrement_register() {
        let mapper = TestMapper::new(vec![0xA2, 0x0f, 0xCA, 0xA0, 0x09, 0x88, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();

        assert_eq!(cpu.register_x, 0x0e);
        assert_eq!(cpu.register_y, 0x08);
    }
    #[test]
    fn test_increment_mem() {
        let mapper = TestMapper::new(vec![0xE6, 0x10, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x10, 0x0e);
        cpu.run_test();
        assert_eq!(cpu.bus.mem_read(0x10), 0x0f);
    }
    #[test]
    fn test_jump_abs() {
        let mapper = TestMapper::new(vec![0x4c, 0x05, 0x80, 0xa9, 0x10, 0xA2, 0x30, 0x00], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();

        assert_ne!(cpu.register_a, 0x10);
        assert_eq!(cpu.register_x, 0x30);
    }
    #[test]
    fn test_pha() {
        let mapper = TestMapper::new(vec![0xa9, 0xe0, 0x48], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);
        
        cpu.run_test();
        assert_eq!(cpu.bus.mem_read((STACK + cpu.stack_pointer as u16).wrapping_add(1)), 0xe0)
    }
    #[test]
    //it doenst purposefully panic anymore
    fn test_pla_at_empty_stack() {
        let mapper = TestMapper::new(vec![0x68, 0x00], Mirroring::Horizontal); 
        let mut cpu = CPU::new(mapper);

        cpu.stack_pointer = 0xFF;

        cpu.bus.mem_write(0x0100, 0xAB);

        cpu.run_test();

        assert_eq!(cpu.register_a, 0xAB);
        assert_eq!(cpu.stack_pointer, 0x00);
    }

    #[test]
    fn test_pla_correctly() {
        let mapper = TestMapper::new(vec![0xa9, 0xe0, 0x48, 0xa9, 0xd0, 0x48, 0xa9, 0x10, 0x68], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();
        assert_eq!(cpu.register_a, 0xd0);
    }
    #[test]
    fn test_eor() {
        let mapper = TestMapper::new(vec![0xa9, 0xA2, 0x45, 0x20], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x20, 0xe0);
        cpu.run_test();
        assert_eq!(cpu.register_a ,0b0100_0010);
    }
    #[test]
    fn test_jsr() {
        let mapper = TestMapper::new(vec![
            0x20, 0x34, 0x12, 0x00  // JSR $1234
        ], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();
        assert_eq!(cpu.program_counter, 0x1234);
        assert_eq!(cpu.stack_pointer, 0xFB);
    }
    #[test]
    fn test_register_write() {
        let mapper = TestMapper::new(vec![0xA2, 0xe0, 0x86, 0x20], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);
        
        cpu.run_test();
        println!("registrador {}",cpu.register_x);
        assert_eq!(cpu.bus.mem_read(0x20), 0xe0);
    }
    #[test]
    fn test_lsr() {
        let mapper = TestMapper::new(vec![0x46, 0x20], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x20, 0xe1);
        cpu.run_test();
        assert!(cpu.status.contains(CpuFlags::CARRY));
        assert_eq!(cpu.bus.mem_read(0x20), 0b0111_0000);
    }
    #[test]
    fn test_ora_from_mem() {
        let mapper = TestMapper::new(vec![0xa9, 0xa1, 0x05, 0x20], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x20, 0xe0);
        cpu.run_test();
        assert_eq!(cpu.register_a, 0b1110_0001);
    }
    #[test]
    fn test_ora_from_immidiate() {
        let mapper = TestMapper::new(vec![0xa9, 0xa1, 0x09, 0xe0], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();
        assert_eq!(cpu.register_a, 0b1110_0001);
    }
    #[test]
    fn test_php() {
        let mapper = TestMapper::new(vec![0x08], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();
        assert_eq!(cpu.stack_pop(), 0b0011_0100);
    }
    #[test]
    fn test_sbc() {
        let mapper = TestMapper::new(vec![0x38, 0xa9, 0xe0, 0xe5, 0x20], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x20, 0x02);
        cpu.run_test();
        assert_eq!(cpu.register_a, 0xde);
    }
    #[test]
    fn test_rti() {
        let mapper = TestMapper::new(vec![0x40], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.stack_push_u16(0x1234);
        cpu.stack_push(0b0010_0100);
        
        cpu.run_test();
        //println!("{}", cpu.program_counter);
        //println!("{}", cpu.status.bits());
        assert_eq!(cpu.program_counter, 0x1234);
        assert_eq!(cpu.status.bits(), 0b0010_0100);
    }
    #[test]
    fn test_rol() {
        let mapper = TestMapper::new(vec![0x26, 0x20], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.bus.mem_write(0x20, 0xe0);
        cpu.run_test();
        assert_eq!(cpu.bus.mem_read(0x20), 0xc0);
    }
    #[test]
    fn test_txa() {
        let mapper = TestMapper::new(vec![0xa2, 0xe0, 0x8a], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();
        assert_eq!(cpu.register_a, 0xe0);
    }
    #[test]
    fn test_tsx() {
        let mapper = TestMapper::new(vec![0xba], Mirroring::Horizontal);
        let mut cpu = CPU::new(mapper);

        cpu.run_test();
        assert_eq!(cpu.register_x, STACK_RESET);
    }
}