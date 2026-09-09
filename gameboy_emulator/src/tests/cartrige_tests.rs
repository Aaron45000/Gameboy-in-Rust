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

#[test]
fn mbc2_selecciona_banco_de_rom_con_4_bits()
{

    let mut cart = new_cartridge(fake_rom(8, 0x05, 0x00, 0x02));

    cart.write_rom(0x2000, 0x03);
    assert_eq!(cart.read_rom(0x4000), 3);

    // el banco 0 se traduce a 1
    cart.write_rom(0x2000, 0x00);
    assert_eq!(cart.read_rom(0x4000), 1);

    // la mitad baja siempre es el banco 0
    assert_eq!(cart.read_rom(0x0000), 0);

}

#[test]
fn mbc2_el_bit_8_de_la_direccion_invalida_la_escritura()
{

    let mut cart = new_cartridge(fake_rom(4, 0x05, 0x00, 0x01));

    // Habilitar RAM: 0x0000 tiene bit 8 = 0, funciona
    cart.write_rom(0x0000, 0x0A);
    cart.write_ram(0xA000, 0x05);
    assert_eq!(cart.read_ram(0xA000), 0xF5, "nibble bajo guardado, alto lee 0xF");

    // 0x0100 tiene bit 8 = 1: la escritura se ignora (no deshabilita la RAM)
    cart.write_rom(0x0100, 0x00);
    assert_eq!(cart.read_ram(0xA000), 0xF5, "el bit 8 en 1 debe ignorar la escritura");

}

#[test]
fn mbc2_la_ram_necesita_estar_habilitada()
{

    let mut cart = new_cartridge(fake_rom(4, 0x05, 0x00, 0x01));

    assert_eq!(cart.read_ram(0xA000), 0xFF, "con la RAM deshabilitada se lee 0xFF");

    cart.write_rom(0x0000, 0x0A);   // habilitar
    cart.write_ram(0xA000, 0x0D);
    assert_eq!(cart.read_ram(0xA000), 0xFD, "solo se guarda el nibble bajo");

}

#[test]
fn mbc2_la_ram_se_espeja_cada_512_direcciones()
{

    let mut cart = new_cartridge(fake_rom(4, 0x05, 0x00, 0x01));

    cart.write_rom(0x0000, 0x0A);
    cart.write_ram(0xA000, 0x03);

    // 0xA000 + 512 = 0xA200 se espeja a la misma posicion
    assert_eq!(cart.read_ram(0xA200), 0xF3, "0xA200 es espejo de 0xA000");

}