use super::Cpu;
use super::R16;

impl Cpu
{
    // --- CALL ---

    pub(super) fn call_imm16(&mut self) -> u8
    {
        let lo  = self.memory.read_byte((self.program_counter + 1) as u16) as u16;
        let hi  = self.memory.read_byte((self.program_counter + 2) as u16) as u16;
        let target = (hi << 8) | lo;

        let return_addr = (self.program_counter.wrapping_add(3)) as u16;
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory.write_byte((self.stack_pointer as usize) as u16, (return_addr >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory.write_byte((self.stack_pointer as usize) as u16, (return_addr & 0xFF) as u8);
        self.program_counter = target as usize;
        return 6;
    }

    pub(super) fn call_cond_imm16(&mut self, bits_543: u8) -> u8
    {
        if self.check_condition(bits_543)
        {
            return self.call_imm16();
        }
        self.program_counter = self.program_counter.wrapping_add(3);
        return 3;
    }


    pub(super) fn ret(&mut self) -> u8
    {
        let lo = self.memory.read_byte((self.stack_pointer as usize) as u16) as usize;
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let hi = self.memory.read_byte((self.stack_pointer as usize) as u16) as usize;
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.program_counter = (hi << 8) | lo;
        return 4;
    }

    pub(super) fn reti(&mut self) -> u8
    {
        self.ime = true;
        return self.ret();
    }

    pub(super) fn ret_cond(&mut self, bits_543: u8) -> u8
    {
        if self.check_condition(bits_543)
        {
            return self.ret().saturating_add(1); 
        }
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    pub(super) fn jp_imm16(&mut self) -> u8
    {
        let lo = self.memory.read_byte((self.program_counter + 1) as u16) as usize;
        let hi = self.memory.read_byte((self.program_counter + 2) as u16) as usize;
        self.program_counter = (hi << 8) | lo;
        return 4;
    }

    pub(super) fn jp_cond_imm16(&mut self, bits_543: u8) -> u8
    {
        if self.check_condition(bits_543)
        {
            return self.jp_imm16();
        }
        self.program_counter = self.program_counter.wrapping_add(3);
        return 3;
    }

    pub(super) fn jp_hl(&mut self) -> u8
    {
        self.program_counter = self.get_r16(R16::HL as u8) as usize;
        return 1;
    }


    pub(super) fn rst(&mut self, bits_543: u8) -> u8
    {
        let vector = (bits_543 as u16) * 8; // vectores: 0x00, 0x08, 0x10 ... 0x38
        let return_addr = (self.program_counter.wrapping_add(1)) as u16;
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory.write_byte((self.stack_pointer as usize) as u16, (return_addr >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory.write_byte((self.stack_pointer as usize) as u16, (return_addr & 0xFF) as u8);
        self.program_counter = vector as usize;
        return 4;
    }

    pub(super) fn jr_imm8(&mut self) -> u8
    {
        
        let offset = self.memory.read_byte((self.program_counter + 1) as u16) as i8;
        self.program_counter = ((self.program_counter as i32) + 2 + (offset as i32)) as usize;
        return 3;
    }

    pub(super) fn jr_cond_imm8(&mut self, bits_543: u8) -> u8
    {
        if self.check_condition(bits_543)
        {
            return self.jr_imm8();
        }

        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }
}
