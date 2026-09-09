use super::Cpu;
use super::R8::{self, A};
use super::R16::{self, HL};

impl Cpu
{
    pub(super) fn ld_highpage_n_a(&mut self) -> u8 // LD [FF00+n], A
    {
        let n    = self.memory.read_byte((self.program_counter + 1) as u16) as usize;
        let addr = 0xFF00 | n;
        self.memory.write_byte((addr) as u16, self.work_registers[A as usize]);
        self.program_counter = self.program_counter.wrapping_add(2);
        return 3;
    }

    pub(super) fn ld_a_highpage_n(&mut self) -> u8 // LD A, [FF00+n]
    {
        let n    = self.memory.read_byte((self.program_counter + 1) as u16) as usize;
        let addr = 0xFF00 | n;
        self.work_registers[A as usize] = self.memory.read_byte((addr) as u16);
        self.program_counter = self.program_counter.wrapping_add(2);
        return 3;
    }

    pub(super) fn ld_highpage_c_a(&mut self) -> u8 // LD [FF00+C], A
    {
        let c    = self.work_registers[R8::C as usize] as usize;
        let addr = 0xFF00 | c;
        self.memory.write_byte((addr) as u16, self.work_registers[A as usize]);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn ld_a_highpage_c(&mut self) -> u8 // LD A, [FF00+C]
    {
        let c    = self.work_registers[R8::C as usize] as usize;
        let addr = 0xFF00 | c;
        self.work_registers[A as usize] = self.memory.read_byte((addr) as u16);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn ld_imm16_a(&mut self) -> u8 // LD [imm16], A
    {
        let lo   = self.memory.read_byte((self.program_counter + 1) as u16) as usize;
        let hi   = self.memory.read_byte((self.program_counter + 2) as u16) as usize;
        let addr = (hi << 8) | lo;
        self.memory.write_byte((addr) as u16, self.work_registers[A as usize]);
        self.program_counter = self.program_counter.wrapping_add(3);
        return 4;
    }

    pub(super) fn ld_a_imm16(&mut self) -> u8 // LD A, [imm16]
    {
        let lo   = self.memory.read_byte((self.program_counter + 1) as u16) as usize;
        let hi   = self.memory.read_byte((self.program_counter + 2) as u16) as usize;
        let addr = (hi << 8) | lo;
        self.work_registers[A as usize] = self.memory.read_byte((addr) as u16);
        self.program_counter = self.program_counter.wrapping_add(3);
        return 4;
    }

    pub(super) fn ld_a_r16mem(&mut self, bits_54: u8) -> u8
    {
        // Obtiene la dirección de memoria según el modo de r16mem;
        // HL+ y HL- post-incrementan/decrementan HL tras leer la dirección.
        let addr = match bits_54
        {
            0b00 => self.get_r16(R16::BC as u8), // [BC]
            0b01 => self.get_r16(R16::DE as u8), // [DE]
            0b10 => // [HL+]
            {
                let addr = self.get_r16(R16::HL as u8);
                self.set_r16(R16::HL as u8, addr.wrapping_add(1));
                addr
            }
            0b11 => // [HL-]
            {
                let addr = self.get_r16(R16::HL as u8);
                self.set_r16(R16::HL as u8, addr.wrapping_sub(1));
                addr
            }
            _ => { println!("todo mal"); return 158; }
        };

        self.work_registers[R8::A as usize] = self.memory.read_byte((addr as usize) as u16);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn ld_r16mem_a(&mut self, bits_54: u8) -> u8
    {

        let a    = self.work_registers[R8::A as usize];
        let addr = match bits_54
        {
            0b00 => self.get_r16(R16::BC as u8), // [BC]
            0b01 => self.get_r16(R16::DE as u8), // [DE]
            0b10 => // [HL+]
            {
                let addr = self.get_r16(R16::HL as u8);
                self.set_r16(R16::HL as u8, addr.wrapping_add(1));
                addr
            }
            0b11 => // [HL-]
            {
                let addr = self.get_r16(R16::HL as u8);
                self.set_r16(R16::HL as u8, addr.wrapping_sub(1));
                addr
            }
            _ => { println!("todo mal"); return 158; }
        };

        self.memory.write_byte((addr as usize) as u16, a);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn ld_r16_imm16(&mut self, bits_54: u8) -> u8
    {
        let lo  = self.memory.read_byte((self.program_counter + 1) as u16) as u16;
        let hi  = self.memory.read_byte((self.program_counter + 2) as u16) as u16;
        let val = (hi << 8) | lo;

        if bits_54 == R16::SP as u8
        {
            self.stack_pointer = val;
        }
        else
        {
            self.set_r16(bits_54, val);
        }

        self.program_counter = self.program_counter.wrapping_add(3);
        return 3;
    }

    pub(super) fn ld_imm16_sp(&mut self) -> u8
    {
        // guarda sp en las direcciones imm16 y imm16 + 1
        let lo   = self.memory.read_byte((self.program_counter + 1) as u16) as u16;
        let hi   = self.memory.read_byte((self.program_counter + 2) as u16) as u16;
        let addr = (hi << 8) | lo;

        self.memory.write_byte((addr as usize) as u16, (self.stack_pointer & 0xFF) as u8);
        self.memory.write_byte((addr as usize + 1) as u16, (self.stack_pointer >> 8) as u8);

        self.program_counter = self.program_counter.wrapping_add(3);
        return 5;
    }

    pub(super) fn ld_r8_imm8(&mut self, bits_543: u8) -> u8
    {

        let imm8 = self.memory.read_byte((self.program_counter + 1) as u16);

        if bits_543 == 0b110
        {

            let address = self.get_r16(R16::HL as u8) as usize;
            self.memory.write_byte((address) as u16, imm8);
            self.program_counter = self.program_counter.wrapping_add(2);
            return 3;

        }

        self.work_registers[bits_543 as usize] = imm8;
        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }

    pub(super) fn ld_r8_r8(&mut self, bits_543: u8, bits_210: u8) -> u8
    {

        if bits_543 == 0b110 // HL como destino
        {

            let addr = self.get_r16(HL as u8);

            self.memory.write_byte((addr as usize) as u16, self.work_registers[bits_210 as usize]);
            self.program_counter = self.program_counter.wrapping_add(1);
            return 2; 
        
        }
        else if bits_210 == 0b110 // HL como fuente
        {

            let addr = self.get_r16(HL as u8);

            self.work_registers[bits_543 as usize] = self.memory.read_byte((addr as usize) as u16);
            self.program_counter = self.program_counter.wrapping_add(1);
            return 2; 
        
        }
        else
        {
            self.work_registers[bits_543 as usize] = self.work_registers[bits_210 as usize];
            self.program_counter = self.program_counter.wrapping_add(1);
            return 1;
        }
    }
}
