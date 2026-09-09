use super::{Mbc, RAM_BANK_SIZE};

pub struct MBC5
{
    rom_data: Vec<Vec<u8>>,   
    ram_data: Vec<Vec<u8>>,   
    rom_banks: usize,
    ram_banks: usize,
    rom_bank_low: u8,
    rom_bank_high: u8,             
    ram_bank: u8,     
    ram_enable: bool,        
    pub has_battery: bool
   
}

impl MBC5
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

        return MBC5 
        {
            rom_data: rom_init,
            ram_data: ram_init,
            rom_banks: rom_banks,
            ram_banks: ram_banks,
            rom_bank_low: 0,
            rom_bank_high: 0,             // bit 8 del banco de ROM (0x3000)
            ram_bank: 0,                  // banco de RAM (0x4000-0x5FFF, 0-15)
            ram_enable: false,
            has_battery: has_battery
            

        }
    }

}

impl Mbc for MBC5
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
            0x2000..=0x2FFF =>
            {

                self.rom_bank_low = value;
                return;

            }
            0x3000..=0x3FFF =>
            {

                self.rom_bank_high = value & 0x01;
                return;

            }
            0x4000..=0x5FFF => 
            {

                self.ram_bank = value & 0x0F;

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
        let current_bank = ((self.rom_bank_high as usize) << 8 | self.rom_bank_low as usize) % self.rom_banks as usize;

        return self.rom_data[current_bank][offset];

    }       

    fn read_ram(&self, address: u16) -> u8
    {

        let offset = address - 0xA000;

        if !self.ram_enable
        {

            return 0xFF;

        }

        if self.ram_banks == 0
        {

            return 0xFF;

        }

        let current_bank = (self.ram_bank as usize) % self.ram_banks;
        return self.ram_data[current_bank][offset as usize];

        
    }

    // Escribe en el banco actual
    fn write_ram(&mut self, address: u16, value: u8)
    {

        if !self.ram_enable || self.ram_banks == 0
        {

            return;

        }

        let offset = (address - 0xA000) as usize;

        let current_bank = (self.ram_bank as usize) % self.ram_banks;
        self.ram_data[current_bank][offset] = value;

    }

    fn step(&mut self, m_cycles: u16)
    {

        return;

    }

    fn save_ram(&self) -> Vec<u8>
    {

        return self.ram_data.concat();

    }

    fn load_ram(&mut self, save: &[u8])
    {

        let mut data = save.to_vec();
        data.resize(RAM_BANK_SIZE*self.ram_banks, 0);
        
        let res = data.chunks(RAM_BANK_SIZE)
        .map(|chunk| chunk.to_vec())
        .collect();
        
        self.ram_data = res;
    }

}
