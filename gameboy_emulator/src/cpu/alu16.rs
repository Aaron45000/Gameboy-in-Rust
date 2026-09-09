use super::Cpu;
use super::R16;

impl Cpu
{
    pub(super) fn push_r16(&mut self, bits_54: u8) -> u8
    {
        let val = if bits_54 == 0b11
        {
            self.get_af() 
        }
        else
        {
            self.get_r16(bits_54)
        };
        let hi = (val >> 8) as u8;
        let lo = (val & 0xFF) as u8;
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory.write_byte((self.stack_pointer as usize) as u16, hi);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory.write_byte((self.stack_pointer as usize) as u16, lo);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 4;
    }

    pub(super) fn pop_r16(&mut self, bits_54: u8) -> u8
    {
        let lo = self.memory.read_byte((self.stack_pointer as usize) as u16) as u16;
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let hi = self.memory.read_byte((self.stack_pointer as usize) as u16) as u16;
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let val = (hi << 8) | lo;
        if bits_54 == 0b11
        {
            self.set_af(val); 
        }
        else
        {
            self.set_r16(bits_54, val);
        }
        self.program_counter = self.program_counter.wrapping_add(1);
        return 3;
    }

    // --- Aritmetica de SP ---

    pub(super) fn add_sp_imm8(&mut self) -> u8 // ADD SP, imm8
    {
        let imm  = self.memory.read_byte((self.program_counter + 1) as u16) as i8;
        let sp   = self.stack_pointer;
        let immu = imm as u16;
        // H y C se calculan sobre el byte bajo de SP
        let halfcarry_flag = (sp & 0xF).wrapping_add(immu & 0xF) > 0xF;
        let carry_flag     = (sp & 0xFF).wrapping_add(immu & 0xFF) > 0xFF;
        self.stack_pointer = ((sp as i32) + (imm as i32)) as u16;
        self.set_flags(false, false, halfcarry_flag, carry_flag);
        self.clear_flags(true, true, !halfcarry_flag, !carry_flag);
        self.program_counter = self.program_counter.wrapping_add(2);
        return 4;
    }

    pub(super) fn ld_hl_sp_imm8(&mut self) -> u8 // LD HL, SP+imm8
    {
        let imm  = self.memory.read_byte((self.program_counter + 1) as u16) as i8;
        let sp   = self.stack_pointer;
        let immu = imm as u16;
        let halfcarry_flag = (sp & 0xF).wrapping_add(immu & 0xF) > 0xF;
        let carry_flag     = (sp & 0xFF).wrapping_add(immu & 0xFF) > 0xFF;
        let result = ((sp as i32) + (imm as i32)) as u16;
        self.set_r16(R16::HL as u8, result);
        self.set_flags(false, false, halfcarry_flag, carry_flag);
        self.clear_flags(true, true, !halfcarry_flag, !carry_flag);
        self.program_counter = self.program_counter.wrapping_add(2);
        return 3;
    }

    pub(super) fn ld_sp_hl(&mut self) -> u8 // LD SP, HL
    {
        self.stack_pointer = self.get_r16(R16::HL as u8);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn add_hl_r16(&mut self, bits_54: u8) -> u8
    {
        let hl = self.get_r16(R16::HL as u8);

        
        let rr:u16;
        if bits_54 == R16::SP as u8
        {
            rr = self.stack_pointer;
        }
        else
        {
            rr = self.get_r16(bits_54)
        }

        let halfcarry_flag = (hl & 0x0FFF) + (rr & 0x0FFF) > 0x0FFF;
        
        let carry_flag= (hl as u32) + (rr as u32) > 0xFFFF;

        self.set_r16(R16::HL as u8, hl.wrapping_add(rr));

        self.set_flags(false, false, halfcarry_flag, carry_flag);
        self.clear_flags(false, true, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn inc_r16(&mut self, bits_54: u8) -> u8
    {
        if bits_54 == R16::SP as u8
        {
            self.stack_pointer = self.stack_pointer.wrapping_add(1);
        }
        else
        {
            let val = self.get_r16(bits_54);
            self.set_r16(bits_54, val.wrapping_add(1));
        }

        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn dec_r16(&mut self, bits_54: u8) -> u8
    {
        if bits_54 == R16::SP as u8
        {
            self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        }
        else
        {
            let val = self.get_r16(bits_54);
            self.set_r16(bits_54, val.wrapping_sub(1));
        }

        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn inc_r8(&mut self, bits_543: u8) -> u8
    {
        if bits_543 == 0b110 // inc [HL]: acceso a memoria en lugar de registro
        {
            let addr = self.get_r16(R16::HL as u8) as usize;
            let old = self.memory.read_byte((addr) as u16);
            let result = old.wrapping_add(1);
            self.memory.write_byte((addr) as u16, result);

            let halfcarry_flag = (old & 0xF) == 0xF;
            let zero_flag = result == 0;

            self.set_flags(zero_flag, false, halfcarry_flag, false);
            self.clear_flags(!zero_flag, true, !halfcarry_flag, false);

            self.program_counter = self.program_counter.wrapping_add(1);
            return 3;
        }

        
        let old    = self.work_registers[bits_543 as usize];
        let result = old.wrapping_add(1);
        self.work_registers[bits_543 as usize] = result;

        let halfcarry_flag = (old & 0xF) == 0xF;
        let zero_flag      = result == 0;

        self.set_flags(zero_flag, false, halfcarry_flag, false);
        self.clear_flags(!zero_flag, true, !halfcarry_flag, false);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    pub(super) fn dec_r8(&mut self, bits_543: u8) -> u8
    {
        if bits_543 == 0b110 // dec [HL]: acceso a memoria en lugar de registro
        {
            let addr= self.get_r16(R16::HL as u8) as usize;
            let old = self.memory.read_byte((addr) as u16);
            let result= old.wrapping_sub(1);
            self.memory.write_byte((addr) as u16, result);

            let halfcarry_flag = (old & 0xF) == 0; // borrow del nibble bajo
            let zero_flag      = result == 0;

            self.set_flags(zero_flag, true, halfcarry_flag, false);
            self.clear_flags(!zero_flag, false, !halfcarry_flag, false);

            self.program_counter = self.program_counter.wrapping_add(1);
            return 3;
        }

        
        let old    = self.work_registers[bits_543 as usize];
        let result = old.wrapping_sub(1);
        self.work_registers[bits_543 as usize] = result;

        let halfcarry_flag = (old & 0xF) == 0; // borrow del nibble bajo
        let zero_flag      = result == 0;

        self.set_flags(zero_flag, true, halfcarry_flag, false);
        self.clear_flags(!zero_flag, false, !halfcarry_flag, false);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }
}
