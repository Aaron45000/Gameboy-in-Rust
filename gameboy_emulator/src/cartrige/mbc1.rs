use super::{Mbc, RAM_BANK_SIZE};

pub struct MBC1
{

    rom_data: Vec<Vec<u8>>,
    ram_data: Vec<Vec<u8>>,
    rom_banks: usize,
    ram_banks: usize,
    bank_low: u8,        
    bank_high: u8,       
    banking_mode: bool,  
    ram_enable: bool,
    pub has_battery: bool

}

impl MBC1
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

            ram_init.push(vec![0u8; RAM_BANK_SIZE]);
            
        }
        return MBC1 {
            rom_data: rom_init,
            ram_data: ram_init,
            rom_banks: rom_banks,
            ram_banks: ram_banks,
            bank_low: 1,
            bank_high: 0,
            banking_mode: false,
            ram_enable: false,
            has_battery: has_battery

        }
    }

}

impl Mbc for MBC1
{

    // Devuelve segun el banco actual
    fn read_rom(&self, address: u16) -> u8
    {

        let offset = (address as usize) & 0x3FFF;

        let mut current_bank = if address < 0x4000
        {

            if self.banking_mode { (self.bank_high as usize) << 5 } else { 0 }
        
        }
        else
        {

            let bank = ((self.bank_high as usize) << 5) | (self.bank_low as usize);
            if bank % 0x20 == 0 { bank + 1 } else { bank }
        
        };

        current_bank %= self.rom_banks;
        return self.rom_data[current_bank][offset];

    }    

    // Configura los registros de bancos
    fn write_rom(&mut self, address: u16, value: u8)
    {

        match address
        {
            0x0000..0x2000 =>
            {

                if (value & 0xF) == 0xA
                {

                    self.ram_enable = true;
                    return;

                }

                self.ram_enable = false;
                return;

            }
            0x2000..0x4000 => 
            {
                
                self.bank_low = value & 0b11111;
                return;

            }
            0x4000..0x6000 =>
            {

                self.bank_high = value & 0b11;
                
            }
            0x6000..0x7FFF =>
            {

                self.banking_mode = value & 0x1 == 1;

            }
            _=> return
        }
        
    }

    // Devuelve el valor en el banco actual
    fn read_ram(&self, address: u16) -> u8
    {

        if !self.ram_enable || self.ram_banks == 0
        {

            return 0xFF;

        }

        let offset = (address - 0xA000) as usize;

        if self.banking_mode
        {

            let current_bank = (self.bank_high as usize) % self.ram_banks;
            return self.ram_data[current_bank][offset];
        }

        let current_bank: usize = 0;
        return self.ram_data[current_bank][offset];

    }

    // Escribe en el banco actual
    fn write_ram(&mut self, address: u16, value: u8)
    {

        if !self.ram_enable || self.ram_banks == 0
        {

            return;

        }

        let offset = (address - 0xA000) as usize;

        if self.banking_mode
        {

            let current_bank = (self.bank_high as usize) % self.ram_banks;
            self.ram_data[current_bank][offset] = value;
            return;

        }

        let current_bank: usize = 0;
        self.ram_data[current_bank][offset] = value;
        return;

    }

    fn step(&mut self, m_cycles: u16)
    {

        return;
        
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
