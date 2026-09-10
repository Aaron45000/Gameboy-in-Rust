use std::cell::Cell;

use super::{Mbc, RAM_BANK_SIZE, T_CYCLES_PER_RTC_SECOND};

pub struct MBC3
{
    rom_data: Vec<Vec<u8>>,   
    ram_data: Vec<Vec<u8>>,   
    rom_banks: usize,
    ram_banks: usize,
    rom_bank: u8,             // registro de 7 bits (0x2000)
    ram_bank: u8,             
    ram_enable: bool,
    rtc_registers: Cell<[u8; 5]>,  
    rtc_cycles: u32,
    latched: [u8; 5],           
    latch_enabled: bool,
    latch_prev: u8,             
    pub has_battery: bool
   
}

impl MBC3
{

    pub fn new(romdata: Vec<u8>, rom_banks: usize, ram_banks: usize, has_battery: bool) -> Self
    {
        
        let mut rom_init:Vec<Vec<u8>> = Vec::with_capacity(rom_banks);
        let mut ram_init:Vec<Vec<u8>> = Vec::with_capacity(ram_banks);


        let mut current_bank = romdata;

        for _ in 0..rom_banks 
        {

            let remaining_data = current_bank.split_off(0x4000); // De esta manera siempre corta pedazos de 0x4000 de tamaño
            rom_init.push(current_bank);
            current_bank = remaining_data;
            
        }

        for _ in 0..ram_banks
        {

            ram_init.push(vec![0u8; 0x2000]);
            
        }

        return MBC3 
        {
            rom_data: rom_init,
            ram_data: ram_init,
            rom_banks: rom_banks,
            ram_banks: ram_banks,
            rom_bank: 0,             // registro de 7 bits (0x2000)
            ram_bank: 0,             // registro de 0x4000 (0-3 o 0x08-0x0C RTC)
            ram_enable: false,
            rtc_registers: Cell::new([0; 5]),
            rtc_cycles: 0,
            latched: [0; 5],           // copia congelada al hacer latch
            latch_enabled: false,
            latch_prev: 0,
            has_battery: has_battery
            

        }
    }

    fn is_halted(&self) -> bool
    {

        return (self.rtc_registers.get()[4] >> 6) & 0x01 == 1;

    }

    fn tick_second(&mut self)
    {

        let mut regs = self.rtc_registers.get();
        let mut days_overflow: bool = false;

        regs[0] = inc_bcd(regs[0]);
        let current_seconds = bcd_to_u8(regs[0]);

        if current_seconds == 60
        {

            regs[0] = 0;
            regs[1] = inc_bcd(regs[1]);

        }

        let current_minutes = bcd_to_u8(regs[1]);

        if current_minutes == 60
        {

            regs[1] = 0;
            regs[2] = inc_bcd(regs[2]);

        }

        let current_hours = bcd_to_u8(regs[2]);

        if current_hours == 24
        {

            regs[2] = 0;
            let (low, overflow) = regs[3].overflowing_add(1);
            regs[3] = low;
            days_overflow = overflow;

        }

        if days_overflow
        {

            if regs[4] & 0x01 == 1
            {

                // 511 -> 512: dias vuelven a 0 y se setea el carry (bit 7)
                regs[4] = (regs[4] & 0xFE) | 0x80;

            }
            else
            {

                // 255 -> 256: se enciende el bit alto de dias
                regs[4] |= 0x01;

            }

        }

        self.rtc_registers.set(regs);
        
    }

}

impl Mbc for MBC3
{

    fn write_rom(&mut self, address: u16, value: u8)
    {

        match address
        {
            0x0000..=0x1FFF =>
            {

                if (value & 0xF) == 0xA
                {

                    self.ram_enable = true;
                    return;

                }

                self.ram_enable = false;
                return;

            }
            0x2000..=0x3FFF =>
            {

                self.rom_bank = value & 0x7F;

            }
            0x4000..=0x5FFF =>
            {

                self.ram_bank = value;

            }
            0x6000..=0x7FFF => 
            {

                if self.latch_prev == 0
                {

                    if value == 1
                    {

                        self.latch_enabled = true;
                        self.latched = self.rtc_registers.get();

                    }
                    else
                    {

                        self.latch_enabled = false;

                    }

                }

                self.latch_prev = value;
                return;

            }
            _=> return
        }
        
    }


    // Devuelve segun el banco actual
    fn read_rom(&self, address: u16) -> u8
    {

        if address < 0x4000
        {

            return self.rom_data[0][address as usize];

        }

        let offset = (address as usize) & 0x3FFF;

        if self.rom_bank == 0
        {

            return self.rom_data[1][offset];
        
        }

        let current_bank = (self.rom_bank as usize) % self.rom_banks;

        return self.rom_data[current_bank][offset];

    }    

    fn read_ram(&self, address: u16) -> u8
    {

        let offset = address - 0xA000;

        if !self.ram_enable
        {

            return 0xFF;

        }

        match self.ram_bank
        {

            0x00..=0x07 =>
            {

                if self.ram_banks == 0
                {

                    return 0xFF;

                }

                let current_bank = (self.ram_bank as usize) % self.ram_banks;
                return self.ram_data[current_bank][offset as usize];

            }
            0x08..=0x0C => 
            {


                let current_register = (self.ram_bank - 0x08) as usize;
                
                let value = if self.latch_enabled
                {

                    self.latched[current_register]

                }
                else 
                {

                    self.rtc_registers.get()[current_register]

                };

                if current_register == 4
                {

                    let mut live = self.rtc_registers.get();
                    live[4] &= 0x7F;   
                    self.rtc_registers.set(live);

                }

                return value;

            }
            _=> 
            {

                return 255;

            }

        }

    }

    fn write_ram(&mut self, address: u16, value: u8)
    {

        let offset = address - 0xA000;

        if !self.ram_enable
        {

            return;

        }

        match self.ram_bank
        {

            0x00..=0x07 =>
            {

                if self.ram_banks == 0
                {

                    return;

                }

                let current_bank = (self.ram_bank as usize) % self.ram_banks;
                self.ram_data[current_bank][offset as usize] = value;
                return;

            }
            0x08..=0x0C => 
            {

                let current_register = (self.ram_bank - 0x08) as usize;

                let mut regs = self.rtc_registers.get();
                regs[current_register] = value;
                self.rtc_registers.set(regs);
                
                if self.latch_enabled
                {

                    self.latched[current_register] = value;

                }

                return;

            }
            _=> 
            {

                return;

            }

        }
    }

    fn step(&mut self, m_cycles: u16)
    {

        if self.is_halted()
        {

            self.rtc_cycles = 0;
            return;

        }

        self.rtc_cycles += 4*m_cycles as u32;

        while self.rtc_cycles >= T_CYCLES_PER_RTC_SECOND
        {

            self.rtc_cycles -= T_CYCLES_PER_RTC_SECOND;
            self.tick_second();

        }

    }

    fn save_ram(&self) -> Vec<u8>
    {

        return self.ram_data.concat();

    }

    fn load_ram(&mut self, save: Vec<u8>)
    {

        let mut data = save;
        data.resize(RAM_BANK_SIZE*self.ram_banks, 0);
        
        let res = data.chunks(RAM_BANK_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect();
        
        self.ram_data = res;

    }
}

fn inc_bcd(bcd: u8) -> u8
{

    let mut low_bcd = bcd & 0x0F;
    let mut high_bcd = (bcd) >> 4;

    low_bcd += 1;

    if low_bcd == 0b1010
    {

        low_bcd = 0; 

        high_bcd += 1;

    }

    if high_bcd == 0b1010
    {

        high_bcd = 0;

    }

    return (high_bcd << 4) | low_bcd;

}

fn bcd_to_u8(bcd: u8) -> u8
{

    let high = bcd >> 4;
    let low = bcd & 0x0F;

    if high > 9 || low > 9 
    {

        return 255;

    }
  
    return high * 10 + low;

}
