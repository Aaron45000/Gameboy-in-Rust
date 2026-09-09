use crate::memory;

#[cfg(test)]
#[path = "tests/cpu_tests.rs"]
mod tests;

mod alu8;
mod alu16;
mod cb;
mod jumps;
mod loads;
mod misc;

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
                                    // stop: ocupa 2 bytes (el segundo se ignora)
                                    self.program_counter = self.program_counter.wrapping_add(2);
                                    return 1;
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

    fn check_condition(&self, bits_543: u8) -> bool
    {
        let f = self.work_registers[R8::F as usize];

        // Solo cuentan los 2 bits bajos: 00=NZ, 01=Z, 10=NC, 11=C.
        // El bloque 0b11 los codifica como 0bcc y el JR como 0b1cc, de ahi la mascara.
        match bits_543 & 0b011
        {
            0b00 => f & 0b10000000 == 0, // NZ
            0b01 => f & 0b10000000 != 0, // Z
            0b10 => f & 0b00010000 == 0, // NC
            _    => f & 0b00010000 != 0, // C
        }
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
