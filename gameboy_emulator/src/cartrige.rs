pub enum MBC_Type { RomOnly = 0,MBC1 = 1, MBC2 = 2, MBC3 = 3, MBC5 = 4, MBC6 = 5, MBC7 = 6, MMM01 = 7, HuC1 = 8, HuC3 = 9 }

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;
const T_CYCLES_PER_RTC_SECOND: u32 = 4_194_304;

pub trait Mbc
{

    fn read_rom(&self, address: u16) -> u8;
    fn write_rom(&mut self, address: u16, value: u8);
    fn read_ram(&self, address: u16) -> u8;
    fn write_ram(&mut self, address: u16, value: u8);
    fn step(&mut self, m_cycles: u16);
    fn save_ram(&self) -> Vec<u8>; // Vec<u8> estado de la ram antes de apagar
    fn load_ram(&mut self, save: &[u8]);

}

mod header;
mod rom_only;
mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;

pub use header::{fetch_ram_banks, fetch_rom_banks, fetch_has_battery, fetch_mbc_type};
pub use rom_only::RomOnly;
pub use mbc1::MBC1;
pub use mbc2::MBC2;
pub use mbc3::MBC3;
pub use mbc5::MBC5;

pub fn new_cartridge(romdata: Vec<u8>) -> Box<dyn Mbc>
{

    let rom_banks = fetch_rom_banks(&romdata);
    let ram_banks = fetch_ram_banks(&romdata);
    let has_battery = fetch_has_battery(&romdata);

    match fetch_mbc_type(&romdata)
    {

        MBC_Type::RomOnly => return Box::new(RomOnly::new(romdata, ram_banks, has_battery)),
        MBC_Type::MBC1    => return Box::new(MBC1::new(romdata, rom_banks, ram_banks, has_battery)),
        MBC_Type::MBC2    => return Box::new(MBC2::new(romdata, rom_banks, has_battery)),
        MBC_Type::MBC3    => return Box::new(MBC3::new(romdata, rom_banks, ram_banks, has_battery)),
        MBC_Type::MBC5    => return Box::new(MBC5::new(romdata, rom_banks, ram_banks, has_battery)),
        _ => panic!("Tipo de MBC no implementado"),

    }
}

fn pad_rom(mut romdata: Vec<u8>, rom_banks: usize) -> Vec<u8>
{

    romdata.resize(rom_banks * ROM_BANK_SIZE, 0xFF);
    return romdata;

}

#[cfg(test)]
#[path = "tests/cartrige_tests.rs"]
mod tests;
