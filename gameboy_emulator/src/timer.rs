use crate::memory::RawMemory;

pub struct Timer {
    pub internal_counter: u16,
    pub last_and_result: bool,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            internal_counter: 0,
            last_and_result: false,
        }
    }

    pub fn step(&mut self, m_cycles: u8, memory: &mut RawMemory) {
        
        let t_cycles = (m_cycles as u16) * 4;

        for _ in 0..t_cycles {
            if memory.div_reset {
                self.internal_counter = 0;
                memory.div_reset = false;
            }

            self.internal_counter = self.internal_counter.wrapping_add(1);

            memory.address_bus[0xFF04] = (self.internal_counter >> 8) as u8;

            let tac = memory.read_byte(0xFF07);
            let timer_enable = (tac & 0b0000_0100) != 0;

            
            let bit_position = match tac & 0b11 {
                0b00 => 9, // 4096 Hz
                0b01 => 3, // 262144 Hz
                0b10 => 5, // 65536 Hz
                0b11 => 7, // 16384 Hz
                _ => unreachable!(),
            };

            let bit_selected = (self.internal_counter & (1 << bit_position)) != 0;

            let current_and_result = bit_selected && timer_enable;

            if self.last_and_result && !current_and_result {
                let tima = memory.read_byte(0xFF05);

                if tima == 0xFF {
            
                    memory.address_bus[0xFF05] = memory.read_byte(0xFF06);
                    
                    let current_if = memory.read_byte(0xFF0F);
                    memory.address_bus[0xFF0F] = current_if | 0b0000_0100;
                } else {
            
                    memory.address_bus[0xFF05] = tima + 1;
                }
            }

            self.last_and_result = current_and_result;
            
        }
    }
}
