use super::Cpu;
use super::R8::{self, A, F};
use super::R16::HL;

impl Cpu
{
    pub(super) fn add_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            let halfcarry_flag = (a & 0xF) + (hl & 0xF) > 0xF;
            let carry_flag     = (a as u16) + (hl as u16) > 0xFF;

            self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_add(hl);
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
            self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        let halfcarry_flag = (a & 0xF) + (r8 & 0xF) > 0xF;
        let carry_flag     = (a as u16) + (r8 as u16) > 0xFF;

        self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_add(r8);
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
        self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    pub(super) fn adc_a_r8 (&mut self, bits_210: u8) -> u8
    {

        let (operand, cycles) = if bits_210 == 0b110
        {
            (self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16), 2)
        }
        else
        {
            (self.work_registers[bits_210 as usize], 1)
        };

        // El carry va sumado aparte: plegarlo en el operando desborda y falsea H y C
        let carry_in: u16 = if self.work_registers[F as usize] & 0b00010000 != 0 { 1 } else { 0 };
        let a = self.work_registers[A as usize] as u16;
        let v = operand as u16;

        let result         = a + v + carry_in;
        let halfcarry_flag = (a & 0xF) + (v & 0xF) + carry_in > 0xF;
        let carry_flag     = result > 0xFF;

        self.work_registers[A as usize] = result as u8;
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
        self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return cycles;
    }

    pub(super) fn sub_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            let halfcarry_flag = (a & 0xF) < (hl & 0xF);
            let carry_flag     = (a as u16) < (hl as u16);

            self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_sub(hl);
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
            self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        let halfcarry_flag = (a & 0xF) < (r8 & 0xF);
        let carry_flag     = (a as u16) < (r8 as u16);

        self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_sub(r8);
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
        self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    pub(super) fn sbc_a_r8 (&mut self, bits_210: u8) -> u8
    {

        let (operand, cycles) = if bits_210 == 0b110
        {
            (self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16), 2)
        }
        else
        {
            (self.work_registers[bits_210 as usize], 1)
        };

        let carry_in: u16 = if self.work_registers[F as usize] & 0b00010000 != 0 { 1 } else { 0 };
        let a = self.work_registers[A as usize] as u16;
        let v = operand as u16;

        let halfcarry_flag = (a & 0xF) < (v & 0xF) + carry_in;
        let carry_flag     = a < v + carry_in;

        self.work_registers[A as usize] = a.wrapping_sub(v).wrapping_sub(carry_in) as u8;
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
        self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return cycles;
    }

    pub(super) fn and_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            self.work_registers[A as usize] = a & hl;
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, true, false);
            self.clear_flags(!zero_flag, true, false, true);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        self.work_registers[A as usize] = a & r8;
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, true, false);
        self.clear_flags(!zero_flag, true, false, true);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    pub(super) fn xor_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            self.work_registers[A as usize] = a ^ hl;
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, false, false);
            self.clear_flags(!zero_flag, true, true, true);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        self.work_registers[A as usize] = a ^ r8;
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, false, false);
        self.clear_flags(!zero_flag, true, true, true);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    pub(super) fn or_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            self.work_registers[A as usize] = a | hl;
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, false, false);
            self.clear_flags(!zero_flag, true, true, true);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        self.work_registers[A as usize] = a | r8;
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, false, false);
        self.clear_flags(!zero_flag, true, true, true);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    pub(super) fn cp_r8(&mut self, bits_210: u8) -> u8
    {
        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            let halfcarry_flag = (a & 0xF) < (hl & 0xF);
            let carry_flag     = (a as u16) < (hl as u16);

            let sub = self.work_registers[A as usize].wrapping_sub(hl);
            let zero_flag = sub == 0;

            self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
            self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        let halfcarry_flag = (a & 0xF) < (r8 & 0xF);
        let carry_flag     = (a as u16) < (r8 as u16);

        let sub = self.work_registers[A as usize].wrapping_sub(r8);
        let zero_flag = sub == 0;

        self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
        self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    pub(super) fn alu_a_imm8(&mut self, bits_543: u8) -> u8
    {
        let imm8 = self.raw_memory.read_byte((self.program_counter + 1) as u16);
        let a    = self.work_registers[A as usize];

        match bits_543
        {
            0b000 => // ADD A, imm8
            {
                let halfcarry_flag = (a & 0xF) + (imm8 & 0xF) > 0xF;
                let carry_flag     = (a as u16) + (imm8 as u16) > 0xFF;
                self.work_registers[A as usize] = a.wrapping_add(imm8);
                let zero_flag = self.work_registers[A as usize] == 0;
                self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
                self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);
            }
            0b001 => // ADC A, imm8
            {
                let carry_in: u16 = if self.work_registers[F as usize] & 0b00010000 != 0 { 1 } else { 0 };
                let result = (a as u16) + (imm8 as u16) + carry_in;
                let halfcarry_flag = (a as u16 & 0xF) + (imm8 as u16 & 0xF) + carry_in > 0xF;
                let carry_flag     = result > 0xFF;
                self.work_registers[A as usize] = result as u8;
                let zero_flag = self.work_registers[A as usize] == 0;
                self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
                self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);
            }
            0b010 => // SUB A, imm8
            {
                let halfcarry_flag = (a & 0xF) < (imm8 & 0xF);
                let carry_flag     = (a as u16) < (imm8 as u16);
                self.work_registers[A as usize] = a.wrapping_sub(imm8);
                let zero_flag = self.work_registers[A as usize] == 0;
                self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
                self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);
            }
            0b011 => // SBC A, imm8
            {
                let carry_in: u16 = if self.work_registers[F as usize] & 0b00010000 != 0 { 1 } else { 0 };
                let halfcarry_flag = (a as u16 & 0xF) < (imm8 as u16 & 0xF) + carry_in;
                let carry_flag     = (a as u16) < (imm8 as u16) + carry_in;
                self.work_registers[A as usize] =
                    (a as u16).wrapping_sub(imm8 as u16).wrapping_sub(carry_in) as u8;
                let zero_flag = self.work_registers[A as usize] == 0;
                self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
                self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);
            }
            0b100 => // AND A, imm8
            {
                self.work_registers[A as usize] = a & imm8;
                let zero_flag = self.work_registers[A as usize] == 0;
                self.set_flags(zero_flag, false, true, false);
                self.clear_flags(!zero_flag, true, false, true);
            }
            0b101 => // XOR A, imm8
            {
                self.work_registers[A as usize] = a ^ imm8;
                let zero_flag = self.work_registers[A as usize] == 0;
                self.set_flags(zero_flag, false, false, false);
                self.clear_flags(!zero_flag, true, true, true);
            }
            0b110 => // OR A, imm8
            {
                self.work_registers[A as usize] = a | imm8;
                let zero_flag = self.work_registers[A as usize] == 0;
                self.set_flags(zero_flag, false, false, false);
                self.clear_flags(!zero_flag, true, true, true);
            }
            0b111 => // CP A, imm8  (compara sin modificar A)
            {
                let halfcarry_flag = (a & 0xF) < (imm8 & 0xF);
                let carry_flag     = (a as u16) < (imm8 as u16);
                let sub            = a.wrapping_sub(imm8);
                let zero_flag      = sub == 0;
                self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
                self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);
            }
            _ =>
            {
                println!("alu_a_imm8: bits_543 invalido: {:03b}", bits_543);
                return 158;
            }
        }

        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }

    pub(super) fn daa(&mut self) -> u8
    {
        let n_flag = self.work_registers[R8::F as usize] & 0b01000000 != 0;
        let h_flag = self.work_registers[R8::F as usize] & 0b00100000 != 0;
        let c_flag = self.work_registers[R8::F as usize] & 0b00010000 != 0;
        let mut a  = self.work_registers[R8::A as usize];
        let mut new_carry = false;

        if !n_flag 
        {
            if c_flag || a > 0x99        { a = a.wrapping_add(0x60); new_carry = true; }
            if h_flag || (a & 0x0F) > 9  { a = a.wrapping_add(0x06); }
        }
        else 
        {
            if c_flag { a = a.wrapping_sub(0x60); new_carry = true; }
            if h_flag { a = a.wrapping_sub(0x06); }
        }

        self.work_registers[R8::A as usize] = a;
        let zero_flag = a == 0;

        self.set_flags(zero_flag, false, false, new_carry);
        self.clear_flags(!zero_flag, false, true, !new_carry);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }
}
