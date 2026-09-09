use super::Cpu;
use super::R8;

impl Cpu
{
    pub(super) fn rlca(&mut self) -> u8
    {
        let a = self.work_registers[R8::A as usize];
        let carry_flag = (a & 0b10000000) != 0;
        let result = (a << 1) | (carry_flag as u8);

        self.work_registers[R8::A as usize] = result;

        self.set_flags(false, false, false, carry_flag);
        self.clear_flags(true, true, true, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }
    pub(super) fn rrca(&mut self) -> u8
    {
        let a = self.work_registers[R8::A as usize];
        let carry_flag = (a & 1) != 0;
        let result = (a >> 1) | ((carry_flag as u8) << 7);

        self.work_registers[R8::A as usize] = result;

        self.set_flags(false, false, false, carry_flag);
        self.clear_flags(true, true, true, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }
    pub(super) fn rla(&mut self) -> u8
    {
        let a = self.work_registers[R8::A as usize];
        let new_carry_flag = (a & 0b10000000) != 0;
        let old_carry_flag = (self.work_registers[R8::F as usize] & 0b00010000) != 0;
        let result = (a << 1) | (old_carry_flag as u8);

        self.work_registers[R8::A as usize] = result;

        self.set_flags(false, false, false, new_carry_flag);
        self.clear_flags(true, true, true, !new_carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }
    pub(super) fn rra(&mut self) -> u8
    {
        let a = self.work_registers[R8::A as usize];
        let new_carry_flag = (a & 1) != 0;
        let old_carry_flag = (self.work_registers[R8::F as usize] & 0b00010000) != 0;
        let result = (a >> 1) | ((old_carry_flag as u8) << 7);

        self.work_registers[R8::A as usize] = result;

        self.set_flags(false, false, false, new_carry_flag);
        self.clear_flags(true, true, true, !new_carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    pub(super) fn cpl(&mut self) -> u8
    {

        self.work_registers[R8::A as usize] = !self.work_registers[R8::A as usize];
        self.set_flags(false, true, true, false);

        self.program_counter = self.program_counter.wrapping_add(1);

        return 1;
    }

    pub(super) fn scf(&mut self) -> u8
    {

        self.set_flags(false, false, false, true);
        self.clear_flags(false, true, true, false); // N y H siempre a cero
        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;

    }

    pub(super) fn ccf(&mut self) -> u8
    {

        if self.work_registers[R8::F as usize] & 0b00010000 != 0
        {
            self.clear_flags(false, false, false, true);
        }
        else
        {
            self.set_flags(false, false, false, true);
        }

        self.clear_flags(false, true, true, false); // N y H siempre a cero

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    
    }

    pub(super) fn halted(&mut self) -> u8
    {
        let pending = self.memory.read_byte((0xFF0F) as u16) & self.memory.read_byte((0xFFFF) as u16);
        if !self.ime && pending != 0 {
            self.halt_bug = true;
        } else {
            self.halted = true;
        }

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }
}
