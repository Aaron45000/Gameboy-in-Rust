use super::Cpu;
use super::{R8, R16};

impl Cpu
{
    pub(super) fn cb_rotates_shifts(&mut self, bits_543: u8, bits_210: u8) -> u8 {
        if bits_210 == 0b110 {
            let addr = self.get_r16(R16::HL as u8) as usize;
            let val = self.memory.read_byte((addr) as u16);
            let new_val = self.perform_cb_rot_shift(bits_543, val);
            self.memory.write_byte((addr) as u16, new_val);
            self.program_counter = self.program_counter.wrapping_add(2);
            return 4;
        }
        
        let val = self.work_registers[bits_210 as usize];
        let new_val = self.perform_cb_rot_shift(bits_543, val);
        self.work_registers[bits_210 as usize] = new_val;
        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }

    pub(super) fn perform_cb_rot_shift(&mut self, bits_543: u8, val: u8) -> u8 {
        let result: u8;
        let mut new_carry = false;
        match bits_543 {
            0b000 => { // RLC
                new_carry = (val & 0b10000000) != 0;
                result = (val << 1) | (if new_carry { 1 } else { 0 });
            },
            0b001 => { // RRC
                new_carry = (val & 1) != 0;
                result = (val >> 1) | (if new_carry { 0b10000000 } else { 0 });
            },
            0b010 => { // RL
                let old_carry = if self.work_registers[R8::F as usize] & 0b00010000 != 0 { 1 } else { 0 };
                new_carry = (val & 0b10000000) != 0;
                result = (val << 1) | old_carry;
            },
            0b011 => { // RR
                let old_carry = if self.work_registers[R8::F as usize] & 0b00010000 != 0 { 0b10000000 } else { 0 };
                new_carry = (val & 1) != 0;
                result = (val >> 1) | old_carry;
            },
            0b100 => { // SLA
                new_carry = (val & 0b10000000) != 0;
                result = val << 1;
            },
            0b101 => { // SRA
                new_carry = (val & 1) != 0;
                result = (val >> 1) | (val & 0b10000000);
            },
            0b110 => { // SWAP
                result = (val << 4) | (val >> 4);
                new_carry = false;
            },
            0b111 => { // SRL
                new_carry = (val & 1) != 0;
                result = val >> 1;
            },
            _ => result = val,
        }
        let zero = result == 0;
        self.set_flags(zero, false, false, new_carry);
        self.clear_flags(!zero, true, true, !new_carry);
        result
    }

    pub(super) fn cb_bit(&mut self, bits_543: u8, bits_210: u8) -> u8 {
        let val = if bits_210 == 0b110 {
            let addr = self.get_r16(R16::HL as u8) as usize;
            self.memory.read_byte((addr) as u16)
        } else {
            self.work_registers[bits_210 as usize]
        };
        
        let zero = (val & (1 << bits_543)) == 0;
        self.set_flags(zero, false, true, false);
        self.clear_flags(!zero, true, false, false);
        
        self.program_counter = self.program_counter.wrapping_add(2);
        if bits_210 == 0b110 { 3 } else { 2 } // Typical timings for BIT
    }

    pub(super) fn cb_res(&mut self, bits_543: u8, bits_210: u8) -> u8 {
        if bits_210 == 0b110 {
            let addr = self.get_r16(R16::HL as u8) as usize;
            let mut val = self.memory.read_byte((addr) as u16);
            val &= !(1 << bits_543);
            self.memory.write_byte((addr) as u16, val);
            self.program_counter = self.program_counter.wrapping_add(2);
            return 4;
        }
        
        let mut val = self.work_registers[bits_210 as usize];
        val &= !(1 << bits_543);
        self.work_registers[bits_210 as usize] = val;
        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }

    pub(super) fn cb_set(&mut self, bits_543: u8, bits_210: u8) -> u8 {
        if bits_210 == 0b110 {
            let addr = self.get_r16(R16::HL as u8) as usize;
            let mut val = self.memory.read_byte((addr) as u16);
            val |= 1 << bits_543;
            self.memory.write_byte((addr) as u16, val);
            self.program_counter = self.program_counter.wrapping_add(2);
            return 4;
        }
        
        let mut val = self.work_registers[bits_210 as usize];
        val |= 1 << bits_543;
        self.work_registers[bits_210 as usize] = val;
        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }
}
