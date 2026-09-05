pub enum MBC_Type { RomOnly = 0,MBC1 = 1, MBC2 = 2, MBC3 = 3, MBC5 = 4, MBC6 = 5, MBC7 = 6, MMM01 = 7, HuC1 = 8, HuC3 = 9 }

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

/// La "interfaz" del cartucho: lo unico que el bus necesita saber hacer.
///
/// Cada implementacion decodifica por su cuenta los registros de control
/// (0x0000-0x7FFF), asi que memory.rs no conoce ningun detalle del chip.
pub trait Mbc
{

    /// Lectura del area de ROM: 0x0000-0x7FFF
    fn read_rom(&self, address: u16) -> u8;

    /// Escritura en el area de ROM: no escribe nada, configura los registros del MBC
    fn write_rom(&mut self, address: u16, value: u8);

    /// Lectura de la RAM externa del cartucho: 0xA000-0xBFFF
    fn read_ram(&self, address: u16) -> u8;

    /// Escritura de la RAM externa del cartucho: 0xA000-0xBFFF
    fn write_ram(&mut self, address: u16, value: u8);

}

/// Construye el cartucho correcto leyendo el header de la ROM.
/// Este es el unico lugar del programa que decide que chip se usa.
pub fn new_cartridge(romdata: Vec<u8>) -> Box<dyn Mbc>
{

    let rom_banks = fetch_rom_banks(&romdata);
    let ram_banks = fetch_ram_banks(&romdata);

    match fetch_mbc_type(&romdata)
    {

        MBC_Type::RomOnly => return Box::new(RomOnly::new(romdata, ram_banks)),
        MBC_Type::MBC1    => return Box::new(MBC1::new(romdata, rom_banks, ram_banks)),
        MBC_Type::MBC3    => return Box::new(MBC3::new(romdata, rom_banks, ram_banks)),
        _ => panic!("Tipo de MBC reconocido pero no implementado"),

    }
}

/// Rellena la ROM hasta el tamano que declara el header, para que ningun
/// indice calculado a partir del numero de bancos se salga del vector.
fn pad_rom(mut romdata: Vec<u8>, rom_banks: usize) -> Vec<u8>
{

    romdata.resize(rom_banks * ROM_BANK_SIZE, 0xFF);
    return romdata;

}


// ---------------------------------------------------------------------------
// ROM ONLY (32 KB, sin mapper)
// ---------------------------------------------------------------------------

pub struct RomOnly
{

    rom_data: Vec<u8>,
    ram_data: Vec<u8>,

}

impl RomOnly
{

    pub fn new(romdata: Vec<u8>, ram_banks: usize) -> Self
    {

        return RomOnly
        {
            rom_data: pad_rom(romdata, 2),
            ram_data: vec![0; ram_banks * RAM_BANK_SIZE],
        };

    }
}

impl Mbc for RomOnly
{

    fn read_rom(&self, address: u16) -> u8
    {

        return self.rom_data[address as usize];

    }

    // Sin mapper no hay registros que configurar: las escrituras se ignoran.
    fn write_rom(&mut self, _address: u16, _value: u8) {}

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
}


// ---------------------------------------------------------------------------
// MBC1
// ---------------------------------------------------------------------------

pub struct MBC1
{

    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    rom_banks: usize,
    ram_banks: usize,
    bank_low: u8,        // registro de 5 bits en 0x2000-0x3FFF
    bank_high: u8,       // registro de 2 bits en 0x4000-0x5FFF
    banking_mode: bool,  // registro de 1 bit en 0x6000-0x7FFF
    ram_enable: bool,

}

impl MBC1
{

    pub fn new(romdata: Vec<u8>, rom_banks: usize, ram_banks: usize) -> Self
    {

        return MBC1
        {
            rom_data: pad_rom(romdata, rom_banks),
            ram_data: vec![0; ram_banks * RAM_BANK_SIZE],
            rom_banks: rom_banks,
            ram_banks: ram_banks,
            bank_low: 1,     // el registro nunca vale 0 en el hardware
            bank_high: 0,
            banking_mode: false,
            ram_enable: false,
        };

    }

    /// El hardware solo tiene cableadas las lineas de direccion que existen,
    /// asi que un banco fuera de rango se envuelve en vez de salirse.
    fn rom_byte(&self, bank: usize, address: u16) -> u8
    {

        let bank = bank % self.rom_banks;
        return self.rom_data[bank * ROM_BANK_SIZE + (address as usize & 0x3FFF)];

    }

    /// Indice plano de la RAM externa, o None si no hay RAM accesible.
    fn ram_index(&self, address: u16) -> Option<usize>
    {

        if !self.ram_enable || self.ram_banks == 0
        {

            return None;

        }

        // El banco de RAM solo se aplica en el modo 1; en el modo 0 siempre es el 0.
        let bank = if self.banking_mode { (self.bank_high as usize) % self.ram_banks } else { 0 };

        return Some(bank * RAM_BANK_SIZE + (address as usize & 0x1FFF));

    }
}

impl Mbc for MBC1
{

    fn read_rom(&self, address: u16) -> u8
    {

        match address
        {

            // En el modo 1 los 2 bits altos tambien afectan a esta mitad
            0x0000..=0x3FFF =>
            {

                let bank = if self.banking_mode { (self.bank_high as usize) << 5 } else { 0 };
                return self.rom_byte(bank, address);

            },

            _ =>
            {

                let bank = ((self.bank_high as usize) << 5) | (self.bank_low as usize);
                return self.rom_byte(bank, address);

            },
        }
    }

    fn write_rom(&mut self, address: u16, value: u8)
    {

        match address
        {

            0x0000..=0x1FFF => self.ram_enable = (value & 0x0F) == 0x0A,

            0x2000..=0x3FFF =>
            {

                // 5 bits; el valor 0 se traduce a 1 (por eso el banco 0 no es seleccionable aqui)
                let mut new_bank = value & 0x1F;

                if new_bank == 0
                {

                    new_bank = 1;

                }

                self.bank_low = new_bank;

            },

            // Este registro se escribe siempre; el modo solo decide como se aplica al leer.
            0x4000..=0x5FFF => self.bank_high = value & 0x03,

            0x6000..=0x7FFF => self.banking_mode = (value & 0x01) == 1,

            _ => {},

        }
    }

    fn read_ram(&self, address: u16) -> u8
    {

        match self.ram_index(address)
        {

            Some(index) => return self.ram_data[index],
            None => return 0xFF,

        }
    }

    fn write_ram(&mut self, address: u16, value: u8)
    {

        if let Some(index) = self.ram_index(address)
        {

            self.ram_data[index] = value;

        }
    }
}


// ---------------------------------------------------------------------------
// MBC3
// ---------------------------------------------------------------------------

pub struct MBC3
{

    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    rom_banks: usize,
    ram_banks: usize,
    rom_bank: u8,        // registro de 7 bits en 0x2000-0x3FFF
    ram_bank: u8,        // 0x00-0x03 banco de RAM, 0x08-0x0C registro del RTC
    ram_enable: bool,
    rtc: [u8; 5],        // RTC sin implementar: los registros existen pero no avanzan
    latch_state: u8,

}

impl MBC3
{

    pub fn new(romdata: Vec<u8>, rom_banks: usize, ram_banks: usize) -> Self
    {

        return MBC3
        {
            rom_data: pad_rom(romdata, rom_banks),
            ram_data: vec![0; ram_banks * RAM_BANK_SIZE],
            rom_banks: rom_banks,
            ram_banks: ram_banks,
            rom_bank: 1,
            ram_bank: 0,
            ram_enable: false,
            rtc: [0; 5],
            latch_state: 0xFF,
        };

    }
}

impl Mbc for MBC3
{

    fn read_rom(&self, address: u16) -> u8
    {

        // A diferencia del MBC1, la mitad baja siempre es el banco 0
        let bank = if address < 0x4000 { 0 } else { (self.rom_bank as usize) % self.rom_banks };

        return self.rom_data[bank * ROM_BANK_SIZE + (address as usize & 0x3FFF)];

    }

    fn write_rom(&mut self, address: u16, value: u8)
    {

        match address
        {

            0x0000..=0x1FFF => self.ram_enable = (value & 0x0F) == 0x0A,

            0x2000..=0x3FFF =>
            {

                // 7 bits, y el 0 se traduce a 1
                let mut new_bank = value & 0x7F;

                if new_bank == 0
                {

                    new_bank = 1;

                }

                self.rom_bank = new_bank;

            },

            0x4000..=0x5FFF => self.ram_bank = value,

            // Latch del reloj: 0x00 seguido de 0x01 congela los registros del RTC
            0x6000..=0x7FFF =>
            {

                self.latch_state = value;

            },

            _ => {},

        }
    }

    fn read_ram(&self, address: u16) -> u8
    {

        if !self.ram_enable
        {

            return 0xFF;

        }

        // 0x08-0x0C mapean los registros del RTC en vez de la RAM
        if self.ram_bank >= 0x08 && self.ram_bank <= 0x0C
        {

            return self.rtc[(self.ram_bank - 0x08) as usize];

        }

        if self.ram_banks == 0
        {

            return 0xFF;

        }

        let bank = (self.ram_bank as usize) % self.ram_banks;

        return self.ram_data[bank * RAM_BANK_SIZE + (address as usize & 0x1FFF)];

    }

    fn write_ram(&mut self, address: u16, value: u8)
    {

        if !self.ram_enable
        {

            return;

        }

        if self.ram_bank >= 0x08 && self.ram_bank <= 0x0C
        {

            self.rtc[(self.ram_bank - 0x08) as usize] = value;
            return;

        }

        if self.ram_banks == 0
        {

            return;

        }

        let bank = (self.ram_bank as usize) % self.ram_banks;
        self.ram_data[bank * RAM_BANK_SIZE + (address as usize & 0x1FFF)] = value;

    }
}


// ---------------------------------------------------------------------------
// Lectura del header
// ---------------------------------------------------------------------------

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

fn fetch_mbc_type(romdata: &[u8]) -> MBC_Type 
{

        let mbc_type_code = romdata[0x147];

        match mbc_type_code {
            0x00 | 0x08 | 0x09 => return MBC_Type::RomOnly, // No MBC
            0x01 | 0x02 | 0x03 => return MBC_Type::MBC1, // MBC1
            0x0F | 0x10 | 0x11 | 0x12 | 0x13 => return MBC_Type::MBC3, // MBC3
            // Los demas no valen la pena
            _ => panic!("Unknown/Unimplemented Cartridge MBC type"),
    }
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests
{

    use super::*;

    /// ROM sintetica: cada banco se rellena con su propio numero de banco,
    /// asi leer un byte dice directamente que banco esta mapeado.
    fn fake_rom(rom_banks: usize, mbc_code: u8, ram_code: u8, rom_code: u8) -> Vec<u8>
    {

        let mut rom = vec![0u8; rom_banks * ROM_BANK_SIZE];

        for bank in 0..rom_banks
        {

            for byte in 0..ROM_BANK_SIZE
            {

                rom[bank * ROM_BANK_SIZE + byte] = bank as u8;

            }
        }

        rom[0x147] = mbc_code;
        rom[0x148] = rom_code;
        rom[0x149] = ram_code;

        return rom;

    }

    #[test]
    fn mbc1_arranca_en_el_banco_1()
    {

        // 4 bancos de ROM (codigo 0x01), sin RAM
        let cart = new_cartridge(fake_rom(4, 0x01, 0x00, 0x01));

        assert_eq!(cart.read_rom(0x0000), 0, "la mitad baja es el banco 0");
        assert_eq!(cart.read_rom(0x4000), 1, "la mitad alta arranca en el banco 1");

    }

    #[test]
    fn mbc1_cambia_de_banco_y_limpia_los_bits_viejos()
    {

        let mut cart = new_cartridge(fake_rom(16, 0x01, 0x00, 0x03));

        cart.write_rom(0x2000, 0x03);
        assert_eq!(cart.read_rom(0x4000), 3);

        // Este era el bug del `|=`: bajar de banco tiene que funcionar
        cart.write_rom(0x2000, 0x02);
        assert_eq!(cart.read_rom(0x4000), 2, "los bits del banco anterior deben limpiarse");

    }

    #[test]
    fn mbc1_el_banco_0_se_traduce_a_1()
    {

        let mut cart = new_cartridge(fake_rom(4, 0x01, 0x00, 0x01));

        cart.write_rom(0x2000, 0x00);
        assert_eq!(cart.read_rom(0x4000), 1);

    }

    #[test]
    fn mbc1_los_bits_altos_seleccionan_bancos_grandes()
    {

        // 64 bancos: hacen falta los 2 bits del registro 0x4000-0x5FFF
        let mut cart = new_cartridge(fake_rom(64, 0x01, 0x00, 0x05));

        cart.write_rom(0x2000, 0x05);   // bits bajos = 5
        cart.write_rom(0x4000, 0x01);   // bits altos = 1 -> banco 0b01_00101 = 37
        assert_eq!(cart.read_rom(0x4000), 37);

    }

    #[test]
    fn mbc1_el_modo_1_reasigna_la_mitad_baja()
    {

        // 128 bancos: el maximo del MBC1, para que el banco 64 exista de verdad
        let mut cart = new_cartridge(fake_rom(128, 0x01, 0x00, 0x06));

        cart.write_rom(0x4000, 0x02);   // bits altos = 2

        // En modo 0 la mitad baja siempre es el banco 0
        assert_eq!(cart.read_rom(0x0000), 0);

        cart.write_rom(0x6000, 0x01);   // modo 1
        assert_eq!(cart.read_rom(0x0000), 64, "modo 1: banco 0b10_00000 = 64");

    }

    #[test]
    fn mbc1_la_ram_necesita_estar_habilitada()
    {

        // 4 bancos de RAM (codigo 0x03)
        let mut cart = new_cartridge(fake_rom(4, 0x02, 0x03, 0x01));

        cart.write_ram(0xA000, 0x42);
        assert_eq!(cart.read_ram(0xA000), 0xFF, "con la RAM deshabilitada se lee 0xFF");

        cart.write_rom(0x0000, 0x0A);   // habilitar RAM
        cart.write_ram(0xA000, 0x42);
        assert_eq!(cart.read_ram(0xA000), 0x42);

        cart.write_rom(0x0000, 0x00);   // deshabilitar
        assert_eq!(cart.read_ram(0xA000), 0xFF);

    }

    #[test]
    fn mbc1_el_banco_de_ram_solo_aplica_en_modo_1()
    {

        let mut cart = new_cartridge(fake_rom(4, 0x02, 0x03, 0x01));

        cart.write_rom(0x0000, 0x0A);   // RAM habilitada
        cart.write_ram(0xA000, 0x11);   // banco 0

        cart.write_rom(0x4000, 0x01);   // registro alto = 1
        assert_eq!(cart.read_ram(0xA000), 0x11, "modo 0: sigue siendo el banco 0");

        cart.write_rom(0x6000, 0x01);   // modo 1 -> ahora si cambia de banco
        cart.write_ram(0xA000, 0x22);
        assert_eq!(cart.read_ram(0xA000), 0x22);

        cart.write_rom(0x4000, 0x00);   // volver al banco 0
        assert_eq!(cart.read_ram(0xA000), 0x11);

    }

    #[test]
    fn mbc3_usa_siete_bits_de_banco()
    {

        // 128 bancos (codigo 0x06), tipo 0x13 = MBC3+RAM+BATTERY
        let mut cart = new_cartridge(fake_rom(128, 0x13, 0x03, 0x06));

        cart.write_rom(0x2000, 0x7F);
        assert_eq!(cart.read_rom(0x4000), 127, "el MBC3 direcciona 127 bancos con un solo registro");

        assert_eq!(cart.read_rom(0x0000), 0, "la mitad baja del MBC3 siempre es el banco 0");

    }

    #[test]
    fn mbc3_mapea_los_registros_del_rtc()
    {

        let mut cart = new_cartridge(fake_rom(8, 0x0F, 0x03, 0x02));

        cart.write_rom(0x0000, 0x0A);   // habilitar RAM/RTC
        cart.write_rom(0x4000, 0x00);
        cart.write_ram(0xA000, 0x55);   // esto va a la RAM

        cart.write_rom(0x4000, 0x08);   // seleccionar registro RTC de segundos
        cart.write_ram(0xA000, 0x30);
        assert_eq!(cart.read_ram(0xA000), 0x30);

        cart.write_rom(0x4000, 0x00);   // volver a la RAM
        assert_eq!(cart.read_ram(0xA000), 0x55, "el RTC no debe pisar la RAM");

    }

    #[test]
    fn romonly_ignora_las_escrituras_de_control()
    {

        let mut cart = new_cartridge(fake_rom(2, 0x00, 0x00, 0x00));

        cart.write_rom(0x2000, 0x05);
        assert_eq!(cart.read_rom(0x4000), 1, "sin mapper la mitad alta siempre es el banco 1");

    }
}
