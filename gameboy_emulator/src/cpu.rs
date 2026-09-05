use crate::{cpu::{R8::{A, F}, R16::HL}, memory};

#[allow(dead_code)]
enum R16 { BC = 0, DE = 1, HL = 2, SP = 3 }

#[allow(dead_code)]
enum R8 { B = 0, C = 1, D = 2, E = 3, H = 4, L = 5, F = 6, A = 7 }


pub struct Cpu
{
    pub program_counter: usize,
    pub stack_pointer: u16,
    pub ime: bool, 
    pub halted: bool,
    pub halt_bug: bool,
    pub work_registers: [u8; 8],
    pub raw_memory: memory::RawMemory,
}

impl Cpu
{
    pub fn new(romdata: Vec<u8>) -> Self
    {
        return Cpu
        {
            program_counter: 0,
            stack_pointer: 0,
            ime: false,
            halted: false,
            halt_bug: false,
            work_registers: [0; 8],
            raw_memory: memory::RawMemory::new(romdata),
        };
    }

    fn get_r16(&self, r16_idx: u8) -> u16
    {
        let i = (r16_idx * 2) as usize;
        (self.work_registers[i] as u16) << 8 | self.work_registers[i + 1] as u16
    }

    fn set_r16(&mut self, r16_idx: u8, val: u16)
    {
        let i = (r16_idx * 2) as usize;
        self.work_registers[i]     = (val >> 8) as u8;
        self.work_registers[i + 1] = (val & 0xFF) as u8;
    }

    fn get_af(&self) -> u16
    {
        (self.work_registers[R8::A as usize] as u16) << 8
            | self.work_registers[R8::F as usize] as u16
    }

    #[allow(dead_code)]
    fn set_af(&mut self, val: u16)
    {
        self.work_registers[R8::A as usize] = (val >> 8) as u8;
        self.work_registers[R8::F as usize] = (val & 0xF0) as u8; // nibble bajo de F siempre 0
    }

    pub fn handle_interrupts(&mut self) -> u8
    {
        let requested = self.raw_memory.read_byte((0xFF0F) as u16);
        let enabled = self.raw_memory.read_byte((0xFFFF) as u16);
        let pending = requested & enabled;

        if pending != 0
        {
            self.halted = false;
        }

        if !self.ime || pending == 0
        {
            return 0;
        }

        for i in 0..=4
        {
            if pending & (1 << i) != 0
            {
                self.ime = false;
                self.raw_memory.write_byte((0xFF0F) as u16, self.raw_memory.read_byte((0xFF0F) as u16) & !(1 << i));

                let hi = (self.program_counter >> 8) as u8;
                let lo = (self.program_counter & 0xFF) as u8; 
 
                self.stack_pointer = self.stack_pointer.wrapping_sub(1);
                self.raw_memory.write_byte((self.stack_pointer as usize) as u16, hi);
                self.stack_pointer = self.stack_pointer.wrapping_sub(1);
                self.raw_memory.write_byte((self.stack_pointer as usize) as u16, lo);

                self.program_counter = match i
                {
                    0 => 0x40, // V-Blank
                    1 => 0x48, // LCD STAT
                    2 => 0x50, // Timer
                    3 => 0x58, // Serial
                    4 => 0x60, // Joypad
                    _ => unreachable!(),
                };

                return 20;
            }
        }
        return 0;
    }

    pub fn step(&mut self) -> u8 // devuelve la cantidad de ticks que usó esa instrucción
    {
        if self.halt_bug {
            self.program_counter = self.program_counter.wrapping_sub(1);
            self.halt_bug = false;
        }

        let opcode   = self.raw_memory.read_byte((self.program_counter) as u16);
        let bits_76  = (self.raw_memory.read_byte((self.program_counter) as u16) & 0xC0) >> 6; // 76
        let bits_3210 = self.raw_memory.read_byte((self.program_counter) as u16) & 0b1111;     // 3210
        let bits_54  = (self.raw_memory.read_byte((self.program_counter) as u16) & 0b110000) >> 4; // 54
        let bits_543 = (self.raw_memory.read_byte((self.program_counter) as u16) & 0b111000) >> 3; // 543
        let bits_210 = self.raw_memory.read_byte((self.program_counter) as u16) & 0b111;          // 210

        if opcode != 0xCB
        {
            match bits_76
            {

                0b00 =>
                {

                    match bits_3210
                    {

                        0b0000 =>
                        {
                            match bits_54
                            {
                                0b00 =>
                                {
                                    self.program_counter = self.program_counter.wrapping_add(1);
                                    return 1;
                                }
                                0b11 | 0b10 =>
                                {
                                    // jr cond, imm8
                                    return self.jr_cond_imm8(bits_543);
                                }
                                0b01 =>
                                {
                                    // stop
                                    return 0;
                                }
                                _ =>
                                {
                                    println!("opcode invalido");
                                    return 158;
                                }
                            }
                        }
                        0b0001 =>
                        {
                            // ld r16, imm16
                            return self.ld_r16_imm16(bits_54);
                        }
                        0b0010 =>
                        {
                            // ld [r16mem], a
                            return self.ld_r16mem_a(bits_54);
                        }
                        0b1010 =>
                        {
                            // ld a, [r16mem]
                            return self.ld_a_r16mem(bits_54);
                        }
                        0b0011 =>
                        {
                            // inc r16
                            return self.inc_r16(bits_54);
                        }
                        0b1011 =>
                        {
                            // dec r16
                            return self.dec_r16(bits_54);
                        }
                        0b1001 =>
                        {
                            // add hl, r16
                            return self.add_hl_r16(bits_54);
                        }
                        0b0100 | 0b1100 =>
                        {
                            // inc r8
                            return self.inc_r8(bits_543);
                        }
                        0b0101 | 0b1101 =>
                        {
                            // dec r8
                            return self.dec_r8(bits_543);
                        }
                        0b0110 | 0b1110 =>
                        {
                            // ld r8, imm8
                            return self.ld_r8_imm8(bits_543);
                        }
                        0b0111 | 0b1111 =>
                        {
                            match bits_543
                            {
                                0b000 => 
                                { 
                                    return self.rlca(); 
                                } // rlca
                                0b001 => 
                                { 
                                    return self.rrca(); 
                                } // rrca
                                0b010 => 
                                { 
                                    return self.rla(); 
                                } // rla
                                0b011 => 
                                { 
                                    return self.rra(); 
                                } // rra
                                0b100 => 
                                { 
                                    return self.daa(); 
                                } // daa
                                0b101 => 
                                { 
                                    return self.cpl(); 
                                } // cpl
                                0b110 => 
                                { 
                                    return self.scf(); 
                                } // scf
                                0b111 => 
                                { 
                                    return self.ccf(); 
                                } // ccf
                                _ =>
                                {
                                    println!("No es un opcode valido");
                                    return 0;
                                }
                            }
                        }
                        0b1000 =>
                        {
                            match bits_54
                            {
                                0b00 =>
                                {
                                    // ld [imm16], sp // guarda sp en las direcciones imm16 y imm16 + 1
                                    return self.ld_imm16_sp();
                                }
                                0b01 =>
                                {
                                    // jr imm8
                                    return self.jr_imm8();
                                }
                                0b11 | 0b10 =>
                                {
                                    // jr cond, imm8
                                    return self.jr_cond_imm8(bits_543);
                                }
                                _ =>
                                {
                                    println!("Registros corruptos");
                                    return 158;
                                }
                            }
                        }
                        _ =>
                        {
                            println!("todo mal");
                            return 158;
                        }

                    }
                }
                0b01 =>
                {
                    if opcode == 0b01110110
                    {
                        // halt
                        return self.halted();
                    }

                    // ld r8, r8
                    return self.ld_r8_r8(bits_543, bits_210);
                }
                0b10 =>
                {
                    match bits_543
                    {
                        0b000 => 
                        {
                            // add a, r8
                            return self.add_a_r8(bits_210); 
                        } 
                        0b001 => 
                        { 
                            // adc a, r8
                            return self.adc_a_r8(bits_210); 
                        } 
                        0b010 => 
                        { 
                            // sub a, r8
                            return self.sub_a_r8(bits_210); 
                        } 
                        0b011 => 
                        { 
                            // sbc a, r8
                            return self.sbc_a_r8(bits_210); 
                        } 
                        0b100 => 
                        { 
                            // and a, r8
                            return self.and_a_r8(bits_210); 
                        } 
                        0b101 => 
                        { 
                            // xor a, r8
                            return self.xor_a_r8(bits_210); 
                        } 
                        0b110 => 
                        { 
                            // or a, r8
                            return self.or_a_r8(bits_210); 
                        } 
                        0b111 => 
                        { 
                            // cp a, r8
                            return self.cp_r8(bits_210); 
                        } 
                        _ =>
                        {
                            println!("registro corruptos");
                            return 158;
                        }
                    }
                }
                0b11 =>
                {
                    match bits_3210
                    {

                        0b0110 | 0b1110 =>
                        {
                            return self.alu_a_imm8(bits_543);
                        }

                        0b0001 => 
                        { 
                            return self.pop_r16(bits_54);  
                        } // POP r16
                        0b0101 => 
                        { 
                            return self.push_r16(bits_54); 
                        } // PUSH r16

                        0b0000 if bits_54 <= 0b01 =>
                        {
                            return self.ret_cond(bits_543); // RET NZ / RET NC
                        }
                        0b1000 if bits_54 <= 0b01 =>
                        {
                            return self.ret_cond(bits_543); // RET Z / RET C
                        }
                        0b1001 if bits_54 == 0b00 => 
                        { 
                            return self.ret();  
                        } // RET
                        0b1001 if bits_54 == 0b01 => 
                        { 
                            return self.reti(); 
                        } // RETI

                        // calls
                        0b0100 if bits_54 <= 0b01 =>
                        {
                            return self.call_cond_imm16(bits_543); // CALL NZ/NC, imm16
                        }
                        0b1100 if bits_54 <= 0b01 =>
                        {
                            return self.call_cond_imm16(bits_543); // CALL Z/C, imm16
                        }
                        0b1101 if bits_54 == 0b00 => 
                        { 
                            return self.call_imm16(); 
                        } // CALL imm16

                        // jp
                        0b0010 if bits_54 <= 0b01 =>
                        {
                            return self.jp_cond_imm16(bits_543); // JP NZ/NC, imm16
                        }
                        0b1010 if bits_54 <= 0b01 =>
                        {
                            return self.jp_cond_imm16(bits_543); // JP Z/C, imm16
                        }
                        0b0011 if bits_54 == 0b00 => 
                        { 
                            return self.jp_imm16(); 
                        } // JP imm16
                        0b1001 if bits_54 == 0b10 => 
                        { 
                            return self.jp_hl();    
                        } // JP HL

                        // RST
                        0b0111 | 0b1111 => { return self.rst(bits_543); }

                        // ldh
                        0b0000 if bits_54 == 0b10 => 
                        { 
                            return self.ld_highpage_n_a();  
                        } // LD [FF00+n], A
                        0b0000 if bits_54 == 0b11 => 
                        { 
                            return self.ld_a_highpage_n();  
                        } // LD A, [FF00+n]
                        0b0010 if bits_54 == 0b10 => 
                        { 
                            return self.ld_highpage_c_a();  
                        } // LD [FF00+C], A
                        0b0010 if bits_54 == 0b11 => 
                        { 
                            return self.ld_a_highpage_c();  
                        } // LD A, [FF00+C]

                        // ---- Loads directos a/desde imm16 ----
                        0b1010 if bits_54 == 0b10 => 
                        { 
                            return self.ld_imm16_a(); 
                        } // LD [imm16], A
                        0b1010 if bits_54 == 0b11 => 
                        { 
                            return self.ld_a_imm16(); 
                        } // LD A, [imm16]

                        // ---- Aritmetica de SP ----
                        0b1000 if bits_54 == 0b10 => 
                        { 
                            return self.add_sp_imm8();    
                        } // ADD SP, imm8
                        0b1000 if bits_54 == 0b11 => 
                        { 
                            return self.ld_hl_sp_imm8(); 
                        } // LD HL, SP+imm8
                        0b1001 if bits_54 == 0b11 => 
                        { 
                            return self.ld_sp_hl();      
                        } // LD SP, HL

                        // ---- EI / DI ----
                        0b0011 if bits_54 == 0b11 => // DI
                        {
                            self.ime = false;
                            self.program_counter = self.program_counter.wrapping_add(1);
                            return 1;
                        }
                        0b1011 if bits_54 == 0b11 => // EI
                        {
                            self.ime = true;
                            self.program_counter = self.program_counter.wrapping_add(1);
                            return 1;
                        }

                        _ =>
                        {
                            println!("bloque 0b11: opcode no cubierto {:02X}", opcode);
                            return 0;
                        }
                    }
                }
                _ =>
                {
                    print!("not covered");
                    return 158;
                }
            }
        }
        else
        {
            let cb_opcode = self.raw_memory.read_byte((self.program_counter + 1) as u16);
            let cb_bits_76 = (cb_opcode & 0xC0) >> 6;
            let cb_bits_543 = (cb_opcode & 0b111000) >> 3;
            let cb_bits_210 = cb_opcode & 0b111;

            match cb_bits_76 {
                0b00 => return self.cb_rotates_shifts(cb_bits_543, cb_bits_210),
                0b01 => return self.cb_bit(cb_bits_543, cb_bits_210),
                0b10 => return self.cb_res(cb_bits_543, cb_bits_210),
                0b11 => return self.cb_set(cb_bits_543, cb_bits_210),
                _    => return 0,
            }
        }
    }

    fn cb_rotates_shifts(&mut self, bits_543: u8, bits_210: u8) -> u8 {
        if bits_210 == 0b110 {
            let addr = self.get_r16(R16::HL as u8) as usize;
            let val = self.raw_memory.read_byte((addr) as u16);
            let new_val = self.perform_cb_rot_shift(bits_543, val);
            self.raw_memory.write_byte((addr) as u16, new_val);
            self.program_counter = self.program_counter.wrapping_add(2);
            return 4;
        }
        
        let val = self.work_registers[bits_210 as usize];
        let new_val = self.perform_cb_rot_shift(bits_543, val);
        self.work_registers[bits_210 as usize] = new_val;
        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }

    fn perform_cb_rot_shift(&mut self, bits_543: u8, val: u8) -> u8 {
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

    fn cb_bit(&mut self, bits_543: u8, bits_210: u8) -> u8 {
        let val = if bits_210 == 0b110 {
            let addr = self.get_r16(R16::HL as u8) as usize;
            self.raw_memory.read_byte((addr) as u16)
        } else {
            self.work_registers[bits_210 as usize]
        };
        
        let zero = (val & (1 << bits_543)) == 0;
        self.set_flags(zero, false, true, false);
        self.clear_flags(!zero, true, false, false);
        
        self.program_counter = self.program_counter.wrapping_add(2);
        if bits_210 == 0b110 { 3 } else { 2 } // Typical timings for BIT
    }

    fn cb_res(&mut self, bits_543: u8, bits_210: u8) -> u8 {
        if bits_210 == 0b110 {
            let addr = self.get_r16(R16::HL as u8) as usize;
            let mut val = self.raw_memory.read_byte((addr) as u16);
            val &= !(1 << bits_543);
            self.raw_memory.write_byte((addr) as u16, val);
            self.program_counter = self.program_counter.wrapping_add(2);
            return 4;
        }
        
        let mut val = self.work_registers[bits_210 as usize];
        val &= !(1 << bits_543);
        self.work_registers[bits_210 as usize] = val;
        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }

    fn cb_set(&mut self, bits_543: u8, bits_210: u8) -> u8 {
        if bits_210 == 0b110 {
            let addr = self.get_r16(R16::HL as u8) as usize;
            let mut val = self.raw_memory.read_byte((addr) as u16);
            val |= 1 << bits_543;
            self.raw_memory.write_byte((addr) as u16, val);
            self.program_counter = self.program_counter.wrapping_add(2);
            return 4;
        }
        
        let mut val = self.work_registers[bits_210 as usize];
        val |= 1 << bits_543;
        self.work_registers[bits_210 as usize] = val;
        self.program_counter = self.program_counter.wrapping_add(2);
        return 2;
    }

    fn check_condition(&self, bits_543: u8) -> bool
    {
        let f = self.work_registers[R8::F as usize];
        match bits_543
        {
            0b100 => f & 0b10000000 == 0, // NZ
            0b101 => f & 0b10000000 != 0, // Z
            0b110 => f & 0b00010000 == 0, // NC
            0b111 => f & 0b00010000 != 0, // C
            _     => { println!("check_condition: bits_543 invalido {:03b}", bits_543); false }
        }
    }

    fn push_r16(&mut self, bits_54: u8) -> u8
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
        self.raw_memory.write_byte((self.stack_pointer as usize) as u16, hi);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.raw_memory.write_byte((self.stack_pointer as usize) as u16, lo);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 4;
    }

    fn pop_r16(&mut self, bits_54: u8) -> u8
    {
        let lo = self.raw_memory.read_byte((self.stack_pointer as usize) as u16) as u16;
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let hi = self.raw_memory.read_byte((self.stack_pointer as usize) as u16) as u16;
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

    // --- CALL ---

    fn call_imm16(&mut self) -> u8
    {
        let lo  = self.raw_memory.read_byte((self.program_counter + 1) as u16) as u16;
        let hi  = self.raw_memory.read_byte((self.program_counter + 2) as u16) as u16;
        let target = (hi << 8) | lo;

        let return_addr = (self.program_counter.wrapping_add(3)) as u16;
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.raw_memory.write_byte((self.stack_pointer as usize) as u16, (return_addr >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.raw_memory.write_byte((self.stack_pointer as usize) as u16, (return_addr & 0xFF) as u8);
        self.program_counter = target as usize;
        return 6;
    }

    fn call_cond_imm16(&mut self, bits_543: u8) -> u8
    {
        if self.check_condition(bits_543)
        {
            return self.call_imm16();
        }
        self.program_counter = self.program_counter.wrapping_add(3);
        return 3;
    }


    fn ret(&mut self) -> u8
    {
        let lo = self.raw_memory.read_byte((self.stack_pointer as usize) as u16) as usize;
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let hi = self.raw_memory.read_byte((self.stack_pointer as usize) as u16) as usize;
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.program_counter = (hi << 8) | lo;
        return 4;
    }

    fn reti(&mut self) -> u8
    {
        self.ime = true;
        return self.ret();
    }

    fn ret_cond(&mut self, bits_543: u8) -> u8
    {
        if self.check_condition(bits_543)
        {
            return self.ret().saturating_add(1); 
        }
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    fn jp_imm16(&mut self) -> u8
    {
        let lo = self.raw_memory.read_byte((self.program_counter + 1) as u16) as usize;
        let hi = self.raw_memory.read_byte((self.program_counter + 2) as u16) as usize;
        self.program_counter = (hi << 8) | lo;
        return 4;
    }

    fn jp_cond_imm16(&mut self, bits_543: u8) -> u8
    {
        if self.check_condition(bits_543)
        {
            return self.jp_imm16();
        }
        self.program_counter = self.program_counter.wrapping_add(3);
        return 3;
    }

    fn jp_hl(&mut self) -> u8
    {
        self.program_counter = self.get_r16(R16::HL as u8) as usize;
        return 1;
    }


    fn rst(&mut self, bits_543: u8) -> u8
    {
        let vector = (bits_543 as u16) * 8; // vectores: 0x00, 0x08, 0x10 ... 0x38
        let return_addr = (self.program_counter.wrapping_add(1)) as u16;
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.raw_memory.write_byte((self.stack_pointer as usize) as u16, (return_addr >> 8) as u8);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.raw_memory.write_byte((self.stack_pointer as usize) as u16, (return_addr & 0xFF) as u8);
        self.program_counter = vector as usize;
        return 4;
    }

    fn ld_highpage_n_a(&mut self) -> u8 // LD [FF00+n], A
    {
        let n    = self.raw_memory.read_byte((self.program_counter + 1) as u16) as usize;
        let addr = 0xFF00 | n;
        self.raw_memory.write_byte((addr) as u16, self.work_registers[A as usize]);
        self.program_counter = self.program_counter.wrapping_add(2);
        return 3;
    }

    fn ld_a_highpage_n(&mut self) -> u8 // LD A, [FF00+n]
    {
        let n    = self.raw_memory.read_byte((self.program_counter + 1) as u16) as usize;
        let addr = 0xFF00 | n;
        self.work_registers[A as usize] = self.raw_memory.read_byte((addr) as u16);
        self.program_counter = self.program_counter.wrapping_add(2);
        return 3;
    }

    fn ld_highpage_c_a(&mut self) -> u8 // LD [FF00+C], A
    {
        let c    = self.work_registers[R8::C as usize] as usize;
        let addr = 0xFF00 | c;
        self.raw_memory.write_byte((addr) as u16, self.work_registers[A as usize]);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    fn ld_a_highpage_c(&mut self) -> u8 // LD A, [FF00+C]
    {
        let c    = self.work_registers[R8::C as usize] as usize;
        let addr = 0xFF00 | c;
        self.work_registers[A as usize] = self.raw_memory.read_byte((addr) as u16);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    fn ld_imm16_a(&mut self) -> u8 // LD [imm16], A
    {
        let lo   = self.raw_memory.read_byte((self.program_counter + 1) as u16) as usize;
        let hi   = self.raw_memory.read_byte((self.program_counter + 2) as u16) as usize;
        let addr = (hi << 8) | lo;
        self.raw_memory.write_byte((addr) as u16, self.work_registers[A as usize]);
        self.program_counter = self.program_counter.wrapping_add(3);
        return 4;
    }

    fn ld_a_imm16(&mut self) -> u8 // LD A, [imm16]
    {
        let lo   = self.raw_memory.read_byte((self.program_counter + 1) as u16) as usize;
        let hi   = self.raw_memory.read_byte((self.program_counter + 2) as u16) as usize;
        let addr = (hi << 8) | lo;
        self.work_registers[A as usize] = self.raw_memory.read_byte((addr) as u16);
        self.program_counter = self.program_counter.wrapping_add(3);
        return 4;
    }

    // --- Aritmetica de SP ---

    fn add_sp_imm8(&mut self) -> u8 // ADD SP, imm8
    {
        let imm  = self.raw_memory.read_byte((self.program_counter + 1) as u16) as i8;
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

    fn ld_hl_sp_imm8(&mut self) -> u8 // LD HL, SP+imm8
    {
        let imm  = self.raw_memory.read_byte((self.program_counter + 1) as u16) as i8;
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

    fn ld_sp_hl(&mut self) -> u8 // LD SP, HL
    {
        self.stack_pointer = self.get_r16(R16::HL as u8);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    fn add_hl_r16(&mut self, bits_54: u8) -> u8
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
        let carry_flag     = (hl as u32) + (rr as u32) > 0xFFFF;

        self.set_r16(R16::HL as u8, hl.wrapping_add(rr));

        self.set_flags(false, false, halfcarry_flag, carry_flag);
        self.clear_flags(false, true, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    fn inc_r16(&mut self, bits_54: u8) -> u8
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

    fn dec_r16(&mut self, bits_54: u8) -> u8
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

    fn ld_a_r16mem(&mut self, bits_54: u8) -> u8
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

        self.work_registers[R8::A as usize] = self.raw_memory.read_byte((addr as usize) as u16);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    fn ld_r16mem_a(&mut self, bits_54: u8) -> u8
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

        self.raw_memory.write_byte((addr as usize) as u16, a);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 2;
    }

    fn ld_r16_imm16(&mut self, bits_54: u8) -> u8
    {
        let lo  = self.raw_memory.read_byte((self.program_counter + 1) as u16) as u16;
        let hi  = self.raw_memory.read_byte((self.program_counter + 2) as u16) as u16;
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

    fn ld_imm16_sp(&mut self) -> u8
    {
        // guarda sp en las direcciones imm16 y imm16 + 1
        let lo   = self.raw_memory.read_byte((self.program_counter + 1) as u16) as u16;
        let hi   = self.raw_memory.read_byte((self.program_counter + 2) as u16) as u16;
        let addr = (hi << 8) | lo;

        self.raw_memory.write_byte((addr as usize) as u16, (self.stack_pointer & 0xFF) as u8);
        self.raw_memory.write_byte((addr as usize + 1) as u16, (self.stack_pointer >> 8) as u8);

        self.program_counter = self.program_counter.wrapping_add(3);
        return 5;
    }

    fn inc_r8(&mut self, bits_543: u8) -> u8
    {
        if bits_543 == 0b110 // inc [HL]: acceso a memoria en lugar de registro
        {
            let addr          = self.get_r16(R16::HL as u8) as usize;
            let old           = self.raw_memory.read_byte((addr) as u16);
            let result        = old.wrapping_add(1);
            self.raw_memory.write_byte((addr) as u16, result);

            let halfcarry_flag = (old & 0xF) == 0xF;
            let zero_flag      = result == 0;

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

    fn dec_r8(&mut self, bits_543: u8) -> u8
    {
        if bits_543 == 0b110 // dec [HL]: acceso a memoria en lugar de registro
        {
            let addr          = self.get_r16(R16::HL as u8) as usize;
            let old           = self.raw_memory.read_byte((addr) as u16);
            let result        = old.wrapping_sub(1);
            self.raw_memory.write_byte((addr) as u16, result);

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

    fn ld_r8_imm8(&mut self, bits_543: u8) -> u8
    {

        if bits_543 == 0b110
        {

            let address = self.get_r16(R16::HL as u8) as usize;
            self.raw_memory.write_byte((address) as u16, self.raw_memory.read_byte((self.program_counter + 1) as u16));
            return 3;

        }
        self.work_registers[bits_543 as usize] = self.raw_memory.read_byte((self.program_counter + 1) as u16);
        return 2;
    }

    fn jr_imm8(&mut self) -> u8
    {
        
        let offset = self.raw_memory.read_byte((self.program_counter + 1) as u16) as i8;
        self.program_counter = ((self.program_counter as i32) + 2 + (offset as i32)) as usize;
        return 3;
    }

    fn jr_cond_imm8(&mut self, bits_543: u8) -> u8
    {

        match bits_543
        {
            0b100 => 
            {
                if self.work_registers[F as usize] & 0b10000000 == 0
                {
                    return self.jr_imm8();
                }       
                self.program_counter = self.program_counter.wrapping_add(2);
                return 2;         
            }
            0b101 => 
            {
                if self.work_registers[F as usize] & 0b10000000 != 0
                {
                    return self.jr_imm8();
                }       
                self.program_counter = self.program_counter.wrapping_add(2);
                return 2;         
            }
            0b110 => 
            {
                if self.work_registers[F as usize] & 0b00010000 == 0
                {
                    return self.jr_imm8();
                }
                self.program_counter = self.program_counter.wrapping_add(2);
                return 2;
            }
            0b111 => 
            {
                if self.work_registers[F as usize] & 0b00010000 != 0
                {
                    return self.jr_imm8();
                }
                self.program_counter = self.program_counter.wrapping_add(2);
                return 2;
            }
            _ => 
            {
                println!("jr_cond_imm8: bits_543 invalido: {:03b}", bits_543);
                return 158;
            }
        }
    }

    fn rlca(&mut self) -> u8
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
    fn rrca(&mut self) -> u8
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
    fn rla(&mut self) -> u8
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
    fn rra(&mut self) -> u8
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

    fn cpl(&mut self) -> u8
    {

        self.work_registers[R8::A as usize] = !self.work_registers[R8::A as usize];
        self.set_flags(false, true, true, false);

        self.program_counter = self.program_counter.wrapping_add(1);

        return 1;
    }

    fn scf(&mut self) -> u8
    {

        self.set_flags(false, false, false, true);
        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;

    }

    fn ccf(&mut self) -> u8
    {

        if self.work_registers[R8::F as usize] & 0b00010000 != 0
        {
            self.clear_flags(false, false, false, true);
        }
        else
        {
            self.set_flags(false, false, false, true);
        }

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    
    }

    fn ld_r8_r8(&mut self, bits_543: u8, bits_210: u8) -> u8
    {

        if bits_543 == 0b110 // HL como destino
        {

            let addr = self.get_r16(HL as u8);

            self.raw_memory.write_byte((addr as usize) as u16, self.work_registers[bits_210 as usize]);
            self.program_counter = self.program_counter.wrapping_add(1);
            return 2; 
        
        }
        else if bits_210 == 0b110 // HL como fuente
        {

            let addr = self.get_r16(HL as u8);

            self.work_registers[bits_543 as usize] = self.raw_memory.read_byte((addr as usize) as u16);
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

    fn add_a_r8 (&mut self, bits_210: u8) -> u8
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

    fn adc_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            let old_carry_flag:u8;
            if self.work_registers[F as usize] & 0b00010000 != 0 
            { 
                old_carry_flag = 1 
            } 
            else 
            { 
                old_carry_flag = 0 
            }

            let r8 = hl + old_carry_flag;

            let halfcarry_flag = (a & 0xF) + (r8 & 0xF) > 0xF;
            let carry_flag     = (a as u16) + (r8 as u16) > 0xFF;

            self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_add(r8);
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
            self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let old_carry_flag:u8;
        if self.work_registers[F as usize] & 0b00010000 != 0 
        {
            old_carry_flag = 1
        }
        else
        {
            old_carry_flag = 0
        }

        let r8 = self.work_registers[bits_210 as usize] + old_carry_flag;


        let halfcarry_flag = (a & 0xF) + (r8 & 0xF) > 0xF;
        let carry_flag     = (a as u16) + (r8 as u16) > 0xFF;

        self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_add(r8);
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
        self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    fn sub_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            let halfcarry_flag = (a & 0xF) < (hl & 0xF);
            let carry_flag     = (a as u16) < (hl as u16);

            self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_sub(hl);
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
            self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);

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

    fn sbc_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            let old_carry_flag:u8;
            if self.work_registers[F as usize] & 0b00010000 != 0 
            { 
                old_carry_flag = 1 
            } 
            else 
            { 
                old_carry_flag = 0 
            }

            let r8 = hl + old_carry_flag;

            let halfcarry_flag = (a & 0xF) < (r8 & 0xF);
            let carry_flag     = (a as u16) < (r8 as u16);

            self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_sub(r8);
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, halfcarry_flag, carry_flag);
            self.clear_flags(!zero_flag, true, !halfcarry_flag, !carry_flag);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let old_carry_flag:u8;
        if self.work_registers[F as usize] & 0b00010000 != 0 
        {
            old_carry_flag = 1
        }
        else
        {
            old_carry_flag = 0
        }

        let r8 = self.work_registers[bits_210 as usize] + old_carry_flag;


        let halfcarry_flag = (a & 0xF) < (r8 & 0xF);
        let carry_flag     = (a as u16) < (r8 as u16);

        self.work_registers[A as usize] = self.work_registers[A as usize].wrapping_sub(r8);
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, true, halfcarry_flag, carry_flag);
        self.clear_flags(!zero_flag, false, !halfcarry_flag, !carry_flag);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    fn and_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            self.work_registers[A as usize] = a & hl;
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, true, false);
            self.clear_flags(!zero_flag, true, false, false);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        self.work_registers[A as usize] = a & r8;
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, true, false);
        self.clear_flags(!zero_flag, true, false, false);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    fn xor_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            self.work_registers[A as usize] = a ^ hl;
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, false, false);
            self.clear_flags(!zero_flag, true, false, false);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        self.work_registers[A as usize] = a ^ r8;
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, false, false);
        self.clear_flags(!zero_flag, true, false, false);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    fn or_a_r8 (&mut self, bits_210: u8) -> u8
    {

        if bits_210 == 0b110
        {

            let hl = self.raw_memory.read_byte((self.get_r16(HL as u8) as usize) as u16);
            let a = self.work_registers[A as usize];

            self.work_registers[A as usize] = a | hl;
            let zero_flag = self.work_registers[A as usize] == 0;

            self.set_flags(zero_flag, false, false, false);
            self.clear_flags(!zero_flag, true, false, false);

            self.program_counter = self.program_counter.wrapping_add(1);
            
            return 2;
        }

        let a = self.work_registers[A as usize];
        let r8 = self.work_registers[bits_210 as usize];

        self.work_registers[A as usize] = a | r8;
        let zero_flag = self.work_registers[A as usize] == 0;

        self.set_flags(zero_flag, false, false, false);
        self.clear_flags(!zero_flag, true, false, false);

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }


    fn cp_r8(&mut self, bits_210: u8) -> u8
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
    fn alu_a_imm8(&mut self, bits_543: u8) -> u8
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
                let carry: u8 = if self.work_registers[F as usize] & 0b00010000 != 0 { 1 } else { 0 };
                let val = imm8.wrapping_add(carry);
                let halfcarry_flag = (a & 0xF) + (val & 0xF) > 0xF;
                let carry_flag     = (a as u16) + (val as u16) > 0xFF;
                self.work_registers[A as usize] = a.wrapping_add(val);
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
                let carry: u8 = if self.work_registers[F as usize] & 0b00010000 != 0 { 1 } else { 0 };
                let val = imm8.wrapping_add(carry);
                let halfcarry_flag = (a & 0xF) < (val & 0xF);
                let carry_flag     = (a as u16) < (val as u16);
                self.work_registers[A as usize] = a.wrapping_sub(val);
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

    
    fn daa(&mut self) -> u8
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

    fn halted(&mut self) -> u8
    {
        let pending = self.raw_memory.read_byte((0xFF0F) as u16) & self.raw_memory.read_byte((0xFFFF) as u16);
        if !self.ime && pending != 0 {
            self.halt_bug = true;
        } else {
            self.halted = true;
        }

        self.program_counter = self.program_counter.wrapping_add(1);
        return 1;
    }

    // Activa los flags indicados en el registro F (bits: Z=7, N=6, H=5, C=4)
    fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool)
    {
        let f = &mut self.work_registers[R8::F as usize];
        if z 
        { 
            *f |= 0b10000000; 
        }
        if n 
        { 
            *f |= 0b01000000; 
        }
        if h 
        { 
            *f |= 0b00100000; 
        }
        if c 
        { 
            *f |= 0b00010000; 
        }
    }

    

    // Limpia los flags indicados en el registro F
    fn clear_flags(&mut self, z: bool, n: bool, h: bool, c: bool)
    {
        let f = &mut self.work_registers[R8::F as usize];
        if z 
        { 
            *f &= !0b10000000; 
        }
        if n 
        { 
            *f &= !0b01000000; 
        }
        if h 
        { 
            *f &= !0b00100000; 
        }
        if c 
        { 
            *f &= !0b00010000; 
        }
    }
}