use super::{Mbc, RAM_BANK_SIZE, pad_rom};

pub struct RomOnly
{

    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    pub has_battery: bool

}

impl RomOnly
{

    pub fn new(romdata: Vec<u8>, _ram_banks: usize, has_battery: bool) -> Self
    {

        return RomOnly
        {
            rom_data: pad_rom(romdata, 2),
            ram_data: vec![0; RAM_BANK_SIZE],
            has_battery: has_battery
        };

    }
}

impl Mbc for RomOnly
{

    fn read_rom(&self, address: u16) -> u8
    {

        return self.rom_data[address as usize];

    }

    fn write_rom(&mut self, _address: u16, _value: u8) 
    {


    }

    fn read_ram(&self, address: u16) -> u8
    {

        let index = (address as usize) - 0xA000;

        if index >= self.ram_data.len()
        {

            return 0xFF;

        }

        return self.ram_data[index];

    }

    fn write_ram(&mut self, address: u16, value: u8)
    {

        let index = (address as usize) - 0xA000;

        if index < self.ram_data.len()
        {

            self.ram_data[index] = value;

        }
    }

    fn step(&mut self, m_cycles: u16)
    {

        return;

    }

    fn save_ram(&self) -> Vec<u8>
    {

        return self.ram_data.clone();

    }

    fn load_ram(&mut self, save: &[u8])
    {

        let mut res  = save.to_vec();
        res.resize(RAM_BANK_SIZE, 0);
        self.ram_data = res;

    }
}
