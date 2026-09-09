use crate::cartrige;

pub struct Memory 
{
    pub address_bus: [u8; 0x10000],
    pub div_reset: bool,
    pub ppu_mode: u8,
    pub cartrige: Box<dyn cartrige::Mbc> // es una caja con una implementacion de Mbc
}

impl Memory
{
    pub fn new(romdata: Vec<u8>) -> Self
    {
        return Memory
        {
            address_bus: [0; 0x10000],
            div_reset: false,
            ppu_mode: 2,
            cartrige: cartrige::new_cartridge(romdata)
        }
    }
    
    pub fn read_byte(&self, address: u16) -> u8 
    {
        match address
        {
            0x0000..=0x7FFF => return self.cartrige.read_rom(address),
            0xA000..=0xBFFF => return self.cartrige.read_ram(address),

            // VRAM bloqueada durante el modo 3
            0x8000..=0x9FFF if self.ppu_mode == 3 => return 0xFF,

            // OAM bloqueada durante los modos 2 y 3
            0xFE00..=0xFE9F if self.ppu_mode == 2 || self.ppu_mode == 3 => return 0xFF,

            _ => {}
        }

        return self.address_bus[address as usize];

    }

    pub fn write_byte(&mut self, address: u16, value: u8) 
    {
        match address 
        {

            0x0000..=0x7FFF => self.cartrige.write_rom(address, value),
            0xA000..=0xBFFF => self.cartrige.write_ram(address, value),
            0x8000..=0x9FFF if self.ppu_mode == 3 => {},
            0xFE00..=0xFE9F if self.ppu_mode == 2 || self.ppu_mode == 3 => {},

            0xFF04 => 
            {

                self.address_bus[0xFF04] = 0;
                self.div_reset = true;
            
            },
            0xFF46 => 
            {
                
                self.address_bus[0xFF46] = value;

                
                let copy_address = (value as u16) << 8;

                for i in 0..160u16
                {

                    let byte = self.dma_source_byte(copy_address + i);
                    self.address_bus[0xFE00 + i as usize] = byte;

                }

            },
            _ => self.address_bus[address as usize] = value,
        }
    }

    /// Lectura para el DMA: ignora los bloqueos de la PPU, como el hardware.
    fn dma_source_byte(&self, address: u16) -> u8
    {

        match address
        {

            0x0000..=0x7FFF => return self.cartrige.read_rom(address),
            0xA000..=0xBFFF => return self.cartrige.read_ram(address),
            _ => return self.address_bus[address as usize],

        }
    }
    
}
