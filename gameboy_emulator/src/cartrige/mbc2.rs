use super::{Mbc, ROM_BANK_SIZE};

pub struct MBC2
{

    rom_data: Vec<Vec<u8>>,
    ram_data: [u8; 512],      // 512 nibbles internos (4 bits cada uno)
    rom_banks: usize,
    rom_bank: u8,             // registro de 4 bits (0x2000-0x3FFF)
    ram_enable: bool,
    has_battery: bool,

}

impl MBC2
{

    pub fn new(romdata: Vec<u8>, rom_banks: usize, has_battery: bool) -> Self
    {

        let mut rom_init: Vec<Vec<u8>> = Vec::with_capacity(rom_banks);

        let mut current_bank = romdata;

        for _ in 0..rom_banks 
        {

            let remaining_data = current_bank.split_off(ROM_BANK_SIZE);
            rom_init.push(current_bank);
            current_bank = remaining_data;
            
        }

        return MBC2
        {
            rom_data: rom_init,
            ram_data: [0; 512],
            rom_banks: rom_banks,
            rom_bank: 0,
            ram_enable: false,
            has_battery: has_battery,

        }
    }

}

impl Mbc for MBC2
{

    fn read_rom(&self, address: u16) -> u8
    {

        if address < 0x4000
        {

            return self.rom_data[0][address as usize];

        }

        let offset = (address as usize) & 0x3FFF;

        let bank = if self.rom_bank == 0 { 1 } else { self.rom_bank as usize };

        let bank = bank % self.rom_banks;

        return self.rom_data[bank][offset];

    }

    fn write_rom(&mut self, address: u16, value: u8)
    {

        if address & 0x0100 == 0x0100 // bit 8 = 1: se ignora la escritura
        {

            return;

        }

        match address
        {
            0x0000..=0x1FFF =>
            {

                self.ram_enable = (value & 0x0F) == 0x0A;

            }
            0x2000..=0x3FFF =>
            {

                self.rom_bank = value & 0x0F;

            }
            _ => {}
        }
        
    }

    fn read_ram(&self, address: u16) -> u8
    {

        if !self.ram_enable
        {

            return 0xFF;

        }

        let index = (address as usize - 0xA000) & 0x1FF;

        return 0xF0 | self.ram_data[index];

    }

    fn write_ram(&mut self, address: u16, value: u8)
    {

        if !self.ram_enable
        {

            return;

        }

        let index = (address as usize - 0xA000) & 0x1FF;

        self.ram_data[index] = value & 0x0F;

    }

    fn step(&mut self, m_cycles: u16)
    {

        return;
        
    }

    fn save_ram(&self) -> Vec<u8>
    {

        return self.ram_data.to_vec();

    }

    fn load_ram(&mut self, save: Vec<u8>)
    {

        let mut data = save;
        data.resize(512, 0);

        for i in 0..512
        {

            self.ram_data[i] = data[i] & 0x0F;

        }

    }
}
