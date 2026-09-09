use super::MBC_Type;

pub fn fetch_ram_banks(romdata: &Vec<u8>) -> usize 
    {

    let ram_size_code = romdata[0x149];

    match ram_size_code { // bancos de 8k
            0x00 => return 0, // No RAM
            0x01 => return 1, // Sin uso oficial, se trata como 1 banco
            0x02 => return 1, // 8 KB RAM, 1 banco
            0x03 => return 4, // 32 KB RAM, 4 bancos
            0x04 => return 16, // 128 KB RAM, 16 bancos
            0x05 => return 8, // 64 KB RAM, 8 bancos
            _ => panic!("Unknown Cartridge RAM size code"),
    }
}

pub fn fetch_rom_banks(romdata: &Vec<u8>) -> usize
{

    let rom_size_code = romdata[0x148];

    match rom_size_code {
            0x00 => return 2, // 32 KB ROM (2 banks)
            0x01 => return 4, // 64 KB ROM (4 banks)
            0x02 => return 8, // 128 KB ROM (8 banks)
            0x03 => return 16, // 256 KB ROM (16 banks)
            0x04 => return 32, // 512 KB ROM (32 banks)
            0x05 => return 64, // 1 MB ROM (64 banks)
            0x06 => return 128, // 2 MB ROM (128 banks)
            0x07 => return 256, // 4 MB ROM (256 banks)
            0x52 => return 72, // 1.1 MB ROM (72 banks)
            0x53 => return 80, // 1.2 MB ROM (80 banks)
            0x54 => return 96, // 1.5 MB ROM (96 banks)
            _ => panic!("Unknown Cartridge ROM size code"),
    }
}


pub fn fetch_has_battery(romdata: &[u8]) -> bool
{

    let mbc_type_code = romdata[0x147];

    match mbc_type_code
    {
        0x03 | 0x06 | 0x09 | 0x0F | 0x10 | 0x13 | 0x1B | 0x1E => return true,
        _=> return false
    }
}

pub fn fetch_mbc_type(romdata: &[u8]) -> MBC_Type 
{

        let mbc_type_code = romdata[0x147];

        match mbc_type_code {
            0x00 | 0x08 | 0x09 => return MBC_Type::RomOnly, // No MBC
            0x01 | 0x02 | 0x03 => return MBC_Type::MBC1, // MBC1
            0x05 | 0x06 => return MBC_Type::MBC2, // MBC2
            0x0F | 0x10 | 0x11 | 0x12 | 0x13 => return MBC_Type::MBC3, // MBC3
            0x19 | 0x1A | 0x1B | 0x1C | 0x1D | 0x1E => return MBC_Type::MBC5, // MBC5
            // Los demas no valen la pena
            _ => panic!("Unknown/Unimplemented Cartridge MBC type"),
        }
}
