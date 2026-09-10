use super::*;

// ---------------------------------------------------------------------------
// Infraestructura
// ---------------------------------------------------------------------------

// Bits del registro F
const FZ: u8 = 0b1000_0000;
const FN: u8 = 0b0100_0000;
const FH: u8 = 0b0010_0000;
const FC: u8 = 0b0001_0000;

// Indices de los registros de 8 bits tal y como los codifican los opcodes.
// El 6 se omite a proposito: en un opcode significa [HL], no un registro.
const RB: u8 = 0;
const RC: u8 = 1;
const RD: u8 = 2;
const RE: u8 = 3;
const RH: u8 = 4;
const RL: u8 = 5;
const RA: u8 = 7;
const REGS: [u8; 7] = [RB, RC, RD, RE, RH, RL, RA];

/// El programa se carga en WRAM porque la ROM ya no es escribible.
const PROG: usize = 0xC000;
/// Zona de datos para las instrucciones que tocan memoria.
const DATA: u16 = 0xC500;
/// Base de pila para los tests de push/pop/call/ret.
const STACK: u16 = 0xC800;

/// CPU con un cartucho minimo y sin bloqueos de la PPU.
fn cpu() -> Cpu
{
    let mut rom = vec![0u8; 2 * 0x4000];
    rom[0x147] = 0x00; // ROM only
    rom[0x148] = 0x00; // 2 bancos
    rom[0x149] = 0x00; // sin RAM

    let mut c = Cpu::new(rom);
    c.memory.ppu_mode = 0; // que VRAM y OAM sean accesibles
    c.stack_pointer = STACK;
    // Estado limpio y determinista: los tests parten de registros en 0,
    // no del estado post-boot.
    c.work_registers = [0; 8];
    c.program_counter = 0;
    return c;
}

/// Escribe el programa en WRAM, apunta PC ahi y ejecuta UNA instruccion.
/// Devuelve los ciclos que reporta la CPU.
fn exec(c: &mut Cpu, program: &[u8]) -> u8
{
    for (i, byte) in program.iter().enumerate()
    {
        c.memory.write_byte((PROG + i) as u16, *byte);
    }
    c.program_counter = PROG;
    return c.step();
}

fn f(c: &Cpu) -> u8
{
    return c.work_registers[R8::F as usize];
}

/// Comprueba los cuatro flags de golpe; da un mensaje legible al fallar.
fn assert_flags(c: &Cpu, z: bool, n: bool, h: bool, carry: bool, ctx: &str)
{
    let got = f(c);
    let want = (if z { FZ } else { 0 }) | (if n { FN } else { 0 })
             | (if h { FH } else { 0 }) | (if carry { FC } else { 0 });

    assert_eq!(got & 0xF0, want,
        "{}: flags esperados Z{} N{} H{} C{}, obtenidos Z{} N{} H{} C{}",
        ctx,
        z as u8, n as u8, h as u8, carry as u8,
        (got & FZ != 0) as u8, (got & FN != 0) as u8,
        (got & FH != 0) as u8, (got & FC != 0) as u8);
}

/// Deja HL apuntando a DATA con `val` dentro.
fn set_hl_mem(c: &mut Cpu, val: u8)
{
    c.set_r16(R16::HL as u8, DATA);
    c.memory.write_byte(DATA, val);
}


// ---------------------------------------------------------------------------
// Bloque 0b00: 0x00 - 0x3F
// ---------------------------------------------------------------------------

#[test]
fn nop()
{
    let mut c = cpu();
    assert_eq!(exec(&mut c, &[0x00]), 1);
    assert_eq!(c.program_counter, PROG + 1);
}

#[test]
fn ld_r16_imm16()
{
    for (op, pair, nombre) in [(0x01u8, R16::BC as u8, "BC"),
                               (0x11,   R16::DE as u8, "DE"),
                               (0x21,   R16::HL as u8, "HL")]
    {
        let mut c = cpu();
        assert_eq!(exec(&mut c, &[op, 0xEF, 0xBE]), 3, "LD {}, imm16", nombre);
        assert_eq!(c.get_r16(pair), 0xBEEF, "LD {}, imm16", nombre);
        assert_eq!(c.program_counter, PROG + 3);
    }

    let mut c = cpu();
    assert_eq!(exec(&mut c, &[0x31, 0xFE, 0xFF]), 3);
    assert_eq!(c.stack_pointer, 0xFFFE, "LD SP, imm16");
}

#[test]
fn ld_r16mem_a()
{
    // [BC] y [DE]
    for (op, pair, nombre) in [(0x02u8, R16::BC as u8, "[BC]"), (0x12, R16::DE as u8, "[DE]")]
    {
        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0x42;
        c.set_r16(pair, DATA);
        assert_eq!(exec(&mut c, &[op]), 2, "LD {}, A", nombre);
        assert_eq!(c.memory.read_byte(DATA), 0x42, "LD {}, A", nombre);
        assert_eq!(c.program_counter, PROG + 1);
    }

    // [HL+]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x11;
    c.set_r16(R16::HL as u8, DATA);
    exec(&mut c, &[0x22]);
    assert_eq!(c.memory.read_byte(DATA), 0x11);
    assert_eq!(c.get_r16(R16::HL as u8), DATA + 1, "[HL+] debe post-incrementar HL");

    // [HL-]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x22;
    c.set_r16(R16::HL as u8, DATA);
    exec(&mut c, &[0x32]);
    assert_eq!(c.memory.read_byte(DATA), 0x22);
    assert_eq!(c.get_r16(R16::HL as u8), DATA - 1, "[HL-] debe post-decrementar HL");
}

#[test]
fn ld_a_r16mem()
{
    for (op, pair, nombre) in [(0x0Au8, R16::BC as u8, "[BC]"), (0x1A, R16::DE as u8, "[DE]")]
    {
        let mut c = cpu();
        c.set_r16(pair, DATA);
        c.memory.write_byte(DATA, 0x99);
        assert_eq!(exec(&mut c, &[op]), 2, "LD A, {}", nombre);
        assert_eq!(c.work_registers[R8::A as usize], 0x99, "LD A, {}", nombre);
    }

    let mut c = cpu();
    set_hl_mem(&mut c, 0x33);
    exec(&mut c, &[0x2A]);
    assert_eq!(c.work_registers[R8::A as usize], 0x33);
    assert_eq!(c.get_r16(R16::HL as u8), DATA + 1);

    let mut c = cpu();
    set_hl_mem(&mut c, 0x44);
    exec(&mut c, &[0x3A]);
    assert_eq!(c.work_registers[R8::A as usize], 0x44);
    assert_eq!(c.get_r16(R16::HL as u8), DATA - 1);
}

#[test]
fn inc_r16()
{
    for (op, pair) in [(0x03u8, R16::BC as u8), (0x13, R16::DE as u8), (0x23, R16::HL as u8)]
    {
        let mut c = cpu();
        c.set_r16(pair, 0x00FF);
        assert_eq!(exec(&mut c, &[op]), 2);
        assert_eq!(c.get_r16(pair), 0x0100, "INC r16 no debe parar en el byte bajo");
    }

    let mut c = cpu();
    c.stack_pointer = 0xFFFF;
    exec(&mut c, &[0x33]);
    assert_eq!(c.stack_pointer, 0x0000, "INC SP debe envolver");

    // INC r16 no toca ningun flag
    let mut c = cpu();
    c.work_registers[R8::F as usize] = 0xF0;
    exec(&mut c, &[0x03]);
    assert_eq!(f(&c), 0xF0, "INC r16 no modifica flags");
}

#[test]
fn dec_r16()
{
    for (op, pair) in [(0x0Bu8, R16::BC as u8), (0x1B, R16::DE as u8), (0x2B, R16::HL as u8)]
    {
        let mut c = cpu();
        c.set_r16(pair, 0x0100);
        assert_eq!(exec(&mut c, &[op]), 2);
        assert_eq!(c.get_r16(pair), 0x00FF);
    }

    let mut c = cpu();
    c.stack_pointer = 0x0000;
    exec(&mut c, &[0x3B]);
    assert_eq!(c.stack_pointer, 0xFFFF, "DEC SP debe envolver");

    let mut c = cpu();
    c.work_registers[R8::F as usize] = 0xF0;
    exec(&mut c, &[0x0B]);
    assert_eq!(f(&c), 0xF0, "DEC r16 no modifica flags");
}

#[test]
fn add_hl_r16()
{
    // Half-carry en el bit 11
    let mut c = cpu();
    c.set_r16(R16::HL as u8, 0x0FFF);
    c.set_r16(R16::BC as u8, 0x0001);
    assert_eq!(exec(&mut c, &[0x09]), 2);
    assert_eq!(c.get_r16(R16::HL as u8), 0x1000);
    assert_flags(&c, false, false, true, false, "ADD HL, BC");

    // Carry en el bit 15
    let mut c = cpu();
    c.set_r16(R16::HL as u8, 0xFFFF);
    c.set_r16(R16::DE as u8, 0x0001);
    exec(&mut c, &[0x19]);
    assert_eq!(c.get_r16(R16::HL as u8), 0x0000);
    assert_flags(&c, false, false, true, true, "ADD HL, DE con acarreo");

    // HL + HL
    let mut c = cpu();
    c.set_r16(R16::HL as u8, 0x1234);
    exec(&mut c, &[0x29]);
    assert_eq!(c.get_r16(R16::HL as u8), 0x2468, "ADD HL, HL");

    // HL + SP
    let mut c = cpu();
    c.set_r16(R16::HL as u8, 0x1000);
    c.stack_pointer = 0x0234;
    exec(&mut c, &[0x39]);
    assert_eq!(c.get_r16(R16::HL as u8), 0x1234, "ADD HL, SP");

    // Z se conserva
    let mut c = cpu();
    c.work_registers[R8::F as usize] = FZ;
    c.set_r16(R16::HL as u8, 0x0001);
    c.set_r16(R16::BC as u8, 0x0001);
    exec(&mut c, &[0x09]);
    assert_eq!(f(&c) & FZ, FZ, "ADD HL, r16 no debe tocar el flag Z");
}

#[test]
fn inc_r8_todos_los_registros()
{
    for r in REGS
    {
        let op = 0b00_000_100 | (r << 3);

        // Half-carry al pasar de 0x0F a 0x10
        let mut c = cpu();
        c.work_registers[r as usize] = 0x0F;
        assert_eq!(exec(&mut c, &[op]), 1, "INC r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0x10, "INC r8 (indice {})", r);
        assert_flags(&c, false, false, true, false, "INC r8 con half-carry");
        assert_eq!(c.program_counter, PROG + 1);

        // Envuelve a cero
        let mut c = cpu();
        c.work_registers[r as usize] = 0xFF;
        exec(&mut c, &[op]);
        assert_eq!(c.work_registers[r as usize], 0x00);
        assert_flags(&c, true, false, true, false, "INC r8 envolviendo");
    }

    // C se conserva
    let mut c = cpu();
    c.work_registers[R8::F as usize] = FC;
    c.work_registers[RB as usize] = 0x01;
    exec(&mut c, &[0x04]);
    assert_eq!(f(&c) & FC, FC, "INC r8 no debe tocar el flag C");
}

#[test]
fn inc_hl_mem()
{
    let mut c = cpu();
    set_hl_mem(&mut c, 0x0F);
    assert_eq!(exec(&mut c, &[0x34]), 3, "INC [HL]");
    assert_eq!(c.memory.read_byte(DATA), 0x10);
    assert_flags(&c, false, false, true, false, "INC [HL]");
}

#[test]
fn dec_r8_todos_los_registros()
{
    for r in REGS
    {
        let op = 0b00_000_101 | (r << 3);

        // Borrow del nibble bajo
        let mut c = cpu();
        c.work_registers[r as usize] = 0x10;
        assert_eq!(exec(&mut c, &[op]), 1, "DEC r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0x0F);
        assert_flags(&c, false, true, true, false, "DEC r8 con borrow");

        // Llega a cero
        let mut c = cpu();
        c.work_registers[r as usize] = 0x01;
        exec(&mut c, &[op]);
        assert_eq!(c.work_registers[r as usize], 0x00);
        assert_flags(&c, true, true, false, false, "DEC r8 a cero");
    }
}

#[test]
fn dec_hl_mem()
{
    let mut c = cpu();
    set_hl_mem(&mut c, 0x00);
    assert_eq!(exec(&mut c, &[0x35]), 3, "DEC [HL]");
    assert_eq!(c.memory.read_byte(DATA), 0xFF);
    assert_flags(&c, false, true, true, false, "DEC [HL] envolviendo");
}

#[test]
fn ld_r8_imm8_todos_los_registros()
{
    for r in REGS
    {
        let op = 0b00_000_110 | (r << 3);
        let mut c = cpu();
        assert_eq!(exec(&mut c, &[op, 0x5A]), 2, "LD r8, imm8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0x5A, "LD r8, imm8 (indice {})", r);
        assert_eq!(c.program_counter, PROG + 2,
            "LD r8, imm8 debe avanzar PC 2 bytes (opcode + inmediato)");
    }
}

#[test]
fn ld_hl_mem_imm8()
{
    let mut c = cpu();
    c.set_r16(R16::HL as u8, DATA);
    assert_eq!(exec(&mut c, &[0x36, 0x7E]), 3, "LD [HL], imm8");
    assert_eq!(c.memory.read_byte(DATA), 0x7E);
    assert_eq!(c.program_counter, PROG + 2, "LD [HL], imm8 debe avanzar PC 2 bytes");
}

#[test]
fn rlca()
{
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0b1000_0001;
    assert_eq!(exec(&mut c, &[0x07]), 1);
    assert_eq!(c.work_registers[R8::A as usize], 0b0000_0011);
    assert_flags(&c, false, false, false, true, "RLCA");
}

#[test]
fn rrca()
{
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0b0000_0011;
    assert_eq!(exec(&mut c, &[0x0F]), 1);
    assert_eq!(c.work_registers[R8::A as usize], 0b1000_0001);
    assert_flags(&c, false, false, false, true, "RRCA");
}

#[test]
fn rla()
{
    // El bit que entra es el carry viejo, no el bit 7
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0b1000_0000;
    c.work_registers[R8::F as usize] = FC;
    assert_eq!(exec(&mut c, &[0x17]), 1);
    assert_eq!(c.work_registers[R8::A as usize], 0b0000_0001);
    assert_flags(&c, false, false, false, true, "RLA");

    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0b0100_0000;
    exec(&mut c, &[0x17]);
    assert_eq!(c.work_registers[R8::A as usize], 0b1000_0000);
    assert_flags(&c, false, false, false, false, "RLA sin carry previo");
}

#[test]
fn rra()
{
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0b0000_0001;
    c.work_registers[R8::F as usize] = FC;
    assert_eq!(exec(&mut c, &[0x1F]), 1);
    assert_eq!(c.work_registers[R8::A as usize], 0b1000_0000);
    assert_flags(&c, false, false, false, true, "RRA");
}

#[test]
fn daa_tras_suma()
{
    // 0x09 + 0x01 = 0x0A -> ajustado a 0x10
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x09;
    exec(&mut c, &[0xC6, 0x01]); // ADD A, 1
    exec(&mut c, &[0x27]);       // DAA
    assert_eq!(c.work_registers[R8::A as usize], 0x10, "DAA tras 09+01");

    // 0x99 + 0x01 = 0x9A -> 0x00 con acarreo decimal
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x99;
    exec(&mut c, &[0xC6, 0x01]);
    exec(&mut c, &[0x27]);
    assert_eq!(c.work_registers[R8::A as usize], 0x00, "DAA tras 99+01");
    assert_flags(&c, true, false, false, true, "DAA con acarreo decimal");
}

#[test]
fn daa_tras_resta()
{
    // 0x10 - 0x01 = 0x0F -> ajustado a 0x09
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x10;
    exec(&mut c, &[0xD6, 0x01]); // SUB A, 1
    exec(&mut c, &[0x27]);       // DAA
    assert_eq!(c.work_registers[R8::A as usize], 0x09, "DAA tras 10-01");
    assert_eq!(f(&c) & FN, FN, "DAA conserva el flag N");
}

#[test]
fn cpl()
{
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0b1010_1010;
    assert_eq!(exec(&mut c, &[0x2F]), 1);
    assert_eq!(c.work_registers[R8::A as usize], 0b0101_0101);
    assert_eq!(f(&c) & (FN | FH), FN | FH, "CPL debe poner N y H");
}

#[test]
fn scf()
{
    let mut c = cpu();
    c.work_registers[R8::F as usize] = FZ | FN | FH;
    assert_eq!(exec(&mut c, &[0x37]), 1);
    assert_flags(&c, true, false, false, true, "SCF debe poner C y limpiar N y H, conservando Z");
}

#[test]
fn ccf()
{
    // C=1 -> C=0
    let mut c = cpu();
    c.work_registers[R8::F as usize] = FN | FH | FC;
    assert_eq!(exec(&mut c, &[0x3F]), 1);
    assert_flags(&c, false, false, false, false, "CCF con C=1 debe limpiar C, N y H");

    // C=0 -> C=1
    let mut c = cpu();
    c.work_registers[R8::F as usize] = FZ;
    exec(&mut c, &[0x3F]);
    assert_flags(&c, true, false, false, true, "CCF con C=0 debe poner C y conservar Z");
}

#[test]
fn ld_imm16_sp()
{
    let mut c = cpu();
    c.stack_pointer = 0xBEEF;
    assert_eq!(exec(&mut c, &[0x08, (DATA & 0xFF) as u8, (DATA >> 8) as u8]), 5);
    assert_eq!(c.memory.read_byte(DATA), 0xEF, "byte bajo de SP");
    assert_eq!(c.memory.read_byte(DATA + 1), 0xBE, "byte alto de SP");
    assert_eq!(c.program_counter, PROG + 3);
}

#[test]
fn jr_imm8()
{
    // Salto hacia adelante: el offset es relativo al final de la instruccion
    let mut c = cpu();
    assert_eq!(exec(&mut c, &[0x18, 0x05]), 3);
    assert_eq!(c.program_counter, PROG + 2 + 5);

    // Salto hacia atras
    let mut c = cpu();
    exec(&mut c, &[0x18, 0xFE]); // -2
    assert_eq!(c.program_counter, PROG, "JR con offset negativo");
}

#[test]
fn jr_cond_imm8()
{
    // (opcode, flag que hace saltar, se salta cuando el flag esta puesto)
    let casos = [(0x20u8, FZ, false), (0x28, FZ, true), (0x30, FC, false), (0x38, FC, true)];

    for (op, flag, salta_con_flag) in casos
    {
        // Caso en el que SI salta
        let mut c = cpu();
        c.work_registers[R8::F as usize] = if salta_con_flag { flag } else { 0 };
        assert_eq!(exec(&mut c, &[op, 0x05]), 3, "JR cond {:02X} tomado", op);
        assert_eq!(c.program_counter, PROG + 2 + 5, "JR cond {:02X} tomado", op);

        // Caso en el que NO salta
        let mut c = cpu();
        c.work_registers[R8::F as usize] = if salta_con_flag { 0 } else { flag };
        assert_eq!(exec(&mut c, &[op, 0x05]), 2, "JR cond {:02X} no tomado", op);
        assert_eq!(c.program_counter, PROG + 2, "JR cond {:02X} no tomado", op);
    }
}

#[test]
fn stop()
{
    let mut c = cpu();
    exec(&mut c, &[0x10, 0x00]);
    assert_eq!(c.program_counter, PROG + 2, "STOP ocupa 2 bytes y debe avanzar PC");
}


// ---------------------------------------------------------------------------
// Bloque 0b01: 0x40 - 0x7F (LD r8, r8 y HALT)
// ---------------------------------------------------------------------------

#[test]
fn ld_r8_r8_todas_las_combinaciones()
{
    for dst in REGS
    {
        for src in REGS
        {
            let mut c = cpu();
            c.work_registers[dst as usize] = 0x00;
            c.work_registers[src as usize] = 0x77;

            let op = 0b01_000_000 | (dst << 3) | src;
            assert_eq!(exec(&mut c, &[op]), 1, "LD r8, r8 ({:02X})", op);
            assert_eq!(c.work_registers[dst as usize], 0x77, "LD r8, r8 ({:02X})", op);
            assert_eq!(c.program_counter, PROG + 1);
        }
    }
}

#[test]
fn ld_hl_mem_r8()
{
    // El valor coincide con los dos bytes de HL a proposito: asi los casos
    // LD [HL], H y LD [HL], L siguen apuntando a la misma direccion de WRAM.
    const ADDR: u16 = 0xC5C5;

    for src in REGS
    {
        let mut c = cpu();
        c.set_r16(R16::HL as u8, ADDR);
        c.work_registers[src as usize] = 0xC5;

        let op = 0b01_110_000 | src;
        assert_eq!(exec(&mut c, &[op]), 2, "LD [HL], r8 ({:02X})", op);
        assert_eq!(c.memory.read_byte(ADDR), 0xC5, "LD [HL], r8 ({:02X})", op);
        assert_eq!(c.program_counter, PROG + 1);
    }
}

#[test]
fn ld_r8_hl_mem()
{
    for dst in REGS
    {
        let mut c = cpu();
        set_hl_mem(&mut c, 0x6D);
        let op = 0b01_000_110 | (dst << 3);
        assert_eq!(exec(&mut c, &[op]), 2, "LD r8, [HL] ({:02X})", op);
        assert_eq!(c.work_registers[dst as usize], 0x6D, "LD r8, [HL] ({:02X})", op);
    }
}

#[test]
fn halt()
{
    // Sin interrupciones pendientes: la CPU se detiene
    let mut c = cpu();
    assert_eq!(exec(&mut c, &[0x76]), 1);
    assert!(c.halted, "HALT sin interrupciones pendientes debe detener la CPU");
    assert_eq!(c.program_counter, PROG + 1);

    // IME=0 con interrupcion pendiente: se dispara el halt bug
    let mut c = cpu();
    c.ime = false;
    c.memory.write_byte(0xFF0F, 0x01);
    c.memory.write_byte(0xFFFF, 0x01);
    exec(&mut c, &[0x76]);
    assert!(c.halt_bug, "HALT con IME=0 e interrupcion pendiente activa el halt bug");
    assert!(!c.halted);
}


// ---------------------------------------------------------------------------
// Bloque 0b10: 0x80 - 0xBF (ALU con registro)
// ---------------------------------------------------------------------------

#[test]
fn add_a_r8()
{
    for r in REGS
    {
        if r == RA { continue; }

        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0x0F;
        c.work_registers[r as usize] = 0x01;

        let op = 0b10_000_000 | r;
        assert_eq!(exec(&mut c, &[op]), 1, "ADD A, r8 ({:02X})", op);
        assert_eq!(c.work_registers[R8::A as usize], 0x10);
        assert_flags(&c, false, false, true, false, "ADD A, r8 con half-carry");
    }

    // ADD A, A
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x80;
    exec(&mut c, &[0x87]);
    assert_eq!(c.work_registers[R8::A as usize], 0x00);
    assert_flags(&c, true, false, false, true, "ADD A, A con acarreo");

    // ADD A, [HL]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0xF0;
    set_hl_mem(&mut c, 0x10);
    assert_eq!(exec(&mut c, &[0x86]), 2, "ADD A, [HL]");
    assert_eq!(c.work_registers[R8::A as usize], 0x00);
    assert_flags(&c, true, false, false, true, "ADD A, [HL] con acarreo");
}

#[test]
fn adc_a_r8()
{
    // Suma sencilla con carry previo
    for r in REGS
    {
        if r == RA { continue; }

        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0x10;
        c.work_registers[r as usize] = 0x01;
        c.work_registers[R8::F as usize] = FC;

        let op = 0b10_001_000 | r;
        assert_eq!(exec(&mut c, &[op]), 1, "ADC A, r8 ({:02X})", op);
        assert_eq!(c.work_registers[R8::A as usize], 0x12, "ADC A, r8 ({:02X})", op);
    }

    // El operando y el carry no se pueden sumar entre si antes de sumarlos a A:
    // 0x00 + 0xFF + 1 = 0x100, no 0x00 + 0x00
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x00;
    c.work_registers[RB as usize] = 0xFF;
    c.work_registers[R8::F as usize] = FC;
    exec(&mut c, &[0x88]);
    assert_eq!(c.work_registers[R8::A as usize], 0x00, "ADC A, B con 0xFF y carry");
    assert_flags(&c, true, false, true, true, "ADC A, B con 0xFF y carry");

    // ADC A, [HL]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x0F;
    c.work_registers[R8::F as usize] = FC;
    set_hl_mem(&mut c, 0x00);
    assert_eq!(exec(&mut c, &[0x8E]), 2, "ADC A, [HL]");
    assert_eq!(c.work_registers[R8::A as usize], 0x10);
    assert_flags(&c, false, false, true, false, "ADC A, [HL] con half-carry");
}

#[test]
fn sub_a_r8()
{
    for r in REGS
    {
        if r == RA { continue; }

        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0x10;
        c.work_registers[r as usize] = 0x01;

        let op = 0b10_010_000 | r;
        assert_eq!(exec(&mut c, &[op]), 1, "SUB A, r8 ({:02X})", op);
        assert_eq!(c.work_registers[R8::A as usize], 0x0F);
        assert_flags(&c, false, true, true, false, "SUB A, r8 con borrow");
    }

    // SUB A, A siempre da cero
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x42;
    exec(&mut c, &[0x97]);
    assert_eq!(c.work_registers[R8::A as usize], 0x00);
    assert_flags(&c, true, true, false, false, "SUB A, A");

    // SUB A, [HL]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x10;
    set_hl_mem(&mut c, 0x01);
    assert_eq!(exec(&mut c, &[0x96]), 2, "SUB A, [HL]");
    assert_eq!(c.work_registers[R8::A as usize], 0x0F);
    assert_flags(&c, false, true, true, false, "SUB A, [HL] debe poner el flag N");
}

#[test]
fn sbc_a_r8()
{
    for r in REGS
    {
        if r == RA { continue; }

        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0x10;
        c.work_registers[r as usize] = 0x00;
        c.work_registers[R8::F as usize] = FC;

        let op = 0b10_011_000 | r;
        assert_eq!(exec(&mut c, &[op]), 1, "SBC A, r8 ({:02X})", op);
        assert_eq!(c.work_registers[R8::A as usize], 0x0F, "SBC A, r8 ({:02X})", op);
        assert_flags(&c, false, true, true, false, "SBC A, r8");
    }

    // Igual que en ADC: el carry no se puede plegar dentro del operando
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x00;
    c.work_registers[RB as usize] = 0xFF;
    c.work_registers[R8::F as usize] = FC;
    exec(&mut c, &[0x98]);
    assert_eq!(c.work_registers[R8::A as usize], 0x00, "SBC A, B con 0xFF y carry");
    assert_flags(&c, true, true, true, true, "SBC A, B con 0xFF y carry");

    // SBC A, [HL]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x10;
    c.work_registers[R8::F as usize] = FC;
    set_hl_mem(&mut c, 0x00);
    assert_eq!(exec(&mut c, &[0x9E]), 2, "SBC A, [HL]");
    assert_eq!(c.work_registers[R8::A as usize], 0x0F);
    assert_flags(&c, false, true, true, false, "SBC A, [HL] debe poner el flag N");
}

#[test]
fn and_a_r8()
{
    for r in REGS
    {
        if r == RA { continue; }

        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0b1100_1100;
        c.work_registers[r as usize] = 0b1010_1010;
        c.work_registers[R8::F as usize] = FC; // AND tiene que limpiarlo

        let op = 0b10_100_000 | r;
        assert_eq!(exec(&mut c, &[op]), 1, "AND A, r8 ({:02X})", op);
        assert_eq!(c.work_registers[R8::A as usize], 0b1000_1000);
        assert_flags(&c, false, false, true, false, "AND A, r8 debe poner H y limpiar C");
    }

    // Resultado cero
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x0F;
    c.work_registers[RB as usize] = 0xF0;
    exec(&mut c, &[0xA0]);
    assert_flags(&c, true, false, true, false, "AND A, B con resultado cero");

    // AND A, [HL]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0xFF;
    c.work_registers[R8::F as usize] = FC;
    set_hl_mem(&mut c, 0x0F);
    assert_eq!(exec(&mut c, &[0xA6]), 2, "AND A, [HL]");
    assert_eq!(c.work_registers[R8::A as usize], 0x0F);
    assert_flags(&c, false, false, true, false, "AND A, [HL] debe limpiar C");
}

#[test]
fn xor_a_r8()
{
    for r in REGS
    {
        if r == RA { continue; }

        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0xFF;
        c.work_registers[r as usize] = 0x0F;
        c.work_registers[R8::F as usize] = FH | FC; // XOR tiene que limpiarlos

        let op = 0b10_101_000 | r;
        assert_eq!(exec(&mut c, &[op]), 1, "XOR A, r8 ({:02X})", op);
        assert_eq!(c.work_registers[R8::A as usize], 0xF0);
        assert_flags(&c, false, false, false, false, "XOR A, r8 debe limpiar N, H y C");
    }

    // XOR A, A es la forma canonica de poner A a cero
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x42;
    exec(&mut c, &[0xAF]);
    assert_eq!(c.work_registers[R8::A as usize], 0x00);
    assert_flags(&c, true, false, false, false, "XOR A, A");

    // XOR A, [HL]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0xF0;
    c.work_registers[R8::F as usize] = FH | FC;
    set_hl_mem(&mut c, 0xFF);
    assert_eq!(exec(&mut c, &[0xAE]), 2, "XOR A, [HL]");
    assert_eq!(c.work_registers[R8::A as usize], 0x0F);
    assert_flags(&c, false, false, false, false, "XOR A, [HL]");
}

#[test]
fn or_a_r8()
{
    for r in REGS
    {
        if r == RA { continue; }

        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0xF0;
        c.work_registers[r as usize] = 0x0F;
        c.work_registers[R8::F as usize] = FH | FC;

        let op = 0b10_110_000 | r;
        assert_eq!(exec(&mut c, &[op]), 1, "OR A, r8 ({:02X})", op);
        assert_eq!(c.work_registers[R8::A as usize], 0xFF);
        assert_flags(&c, false, false, false, false, "OR A, r8 debe limpiar N, H y C");
    }

    // Resultado cero
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x00;
    c.work_registers[RB as usize] = 0x00;
    exec(&mut c, &[0xB0]);
    assert_flags(&c, true, false, false, false, "OR A, B con resultado cero");

    // OR A, [HL]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0xF0;
    set_hl_mem(&mut c, 0x0F);
    assert_eq!(exec(&mut c, &[0xB6]), 2, "OR A, [HL]");
    assert_eq!(c.work_registers[R8::A as usize], 0xFF);
}

#[test]
fn cp_a_r8()
{
    for r in REGS
    {
        if r == RA { continue; }

        // Iguales -> Z
        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0x42;
        c.work_registers[r as usize] = 0x42;

        let op = 0b10_111_000 | r;
        assert_eq!(exec(&mut c, &[op]), 1, "CP A, r8 ({:02X})", op);
        assert_eq!(c.work_registers[R8::A as usize], 0x42, "CP no debe modificar A");
        assert_flags(&c, true, true, false, false, "CP A, r8 con valores iguales");

        // A menor -> C
        let mut c = cpu();
        c.work_registers[R8::A as usize] = 0x00;
        c.work_registers[r as usize] = 0x01;
        exec(&mut c, &[op]);
        assert_flags(&c, false, true, true, true, "CP A, r8 con A menor");
    }

    // CP A, A
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x33;
    exec(&mut c, &[0xBF]);
    assert_flags(&c, true, true, false, false, "CP A, A");

    // CP A, [HL]
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x10;
    set_hl_mem(&mut c, 0x10);
    assert_eq!(exec(&mut c, &[0xBE]), 2, "CP A, [HL]");
    assert_flags(&c, true, true, false, false, "CP A, [HL]");
}


// ---------------------------------------------------------------------------
// Bloque 0b11: 0xC0 - 0xFF
// ---------------------------------------------------------------------------

#[test]
fn alu_a_imm8()
{
    // ADD A, imm8
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x0F;
    assert_eq!(exec(&mut c, &[0xC6, 0x01]), 2, "ADD A, imm8");
    assert_eq!(c.work_registers[R8::A as usize], 0x10);
    assert_flags(&c, false, false, true, false, "ADD A, imm8");
    assert_eq!(c.program_counter, PROG + 2);

    // SUB A, imm8
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x10;
    exec(&mut c, &[0xD6, 0x01]);
    assert_eq!(c.work_registers[R8::A as usize], 0x0F);
    assert_flags(&c, false, true, true, false, "SUB A, imm8");

    // AND A, imm8
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0xFF;
    c.work_registers[R8::F as usize] = FC;
    exec(&mut c, &[0xE6, 0x0F]);
    assert_eq!(c.work_registers[R8::A as usize], 0x0F);
    assert_flags(&c, false, false, true, false, "AND A, imm8");

    // XOR A, imm8
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0xFF;
    exec(&mut c, &[0xEE, 0x0F]);
    assert_eq!(c.work_registers[R8::A as usize], 0xF0);
    assert_flags(&c, false, false, false, false, "XOR A, imm8");

    // OR A, imm8
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0xF0;
    exec(&mut c, &[0xF6, 0x0F]);
    assert_eq!(c.work_registers[R8::A as usize], 0xFF);
    assert_flags(&c, false, false, false, false, "OR A, imm8");

    // CP A, imm8
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x42;
    exec(&mut c, &[0xFE, 0x42]);
    assert_eq!(c.work_registers[R8::A as usize], 0x42, "CP no modifica A");
    assert_flags(&c, true, true, false, false, "CP A, imm8");
}

#[test]
fn adc_sbc_a_imm8()
{
    // ADC A, imm8 sencillo
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x10;
    c.work_registers[R8::F as usize] = FC;
    assert_eq!(exec(&mut c, &[0xCE, 0x01]), 2, "ADC A, imm8");
    assert_eq!(c.work_registers[R8::A as usize], 0x12);

    // 0x0F + 0xFF + 1 = 0x10F: el resultado es 0x0F pero con H y C puestos
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x0F;
    c.work_registers[R8::F as usize] = FC;
    exec(&mut c, &[0xCE, 0xFF]);
    assert_eq!(c.work_registers[R8::A as usize], 0x0F, "ADC A, 0xFF con carry");
    assert_flags(&c, false, false, true, true, "ADC A, 0xFF con carry");

    // SBC A, imm8 sencillo
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x10;
    c.work_registers[R8::F as usize] = FC;
    exec(&mut c, &[0xDE, 0x00]);
    assert_eq!(c.work_registers[R8::A as usize], 0x0F, "SBC A, imm8");
    assert_flags(&c, false, true, true, false, "SBC A, imm8");

    // 0x00 - 0xFF - 1 = -0x100: resultado 0x00 con H y C puestos
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x00;
    c.work_registers[R8::F as usize] = FC;
    exec(&mut c, &[0xDE, 0xFF]);
    assert_eq!(c.work_registers[R8::A as usize], 0x00, "SBC A, 0xFF con carry");
    assert_flags(&c, true, true, true, true, "SBC A, 0xFF con carry");
}

#[test]
fn push_r16()
{
    for (op, pair, nombre) in [(0xC5u8, R16::BC as u8, "BC"),
                               (0xD5,   R16::DE as u8, "DE"),
                               (0xE5,   R16::HL as u8, "HL")]
    {
        let mut c = cpu();
        c.set_r16(pair, 0x1234);
        assert_eq!(exec(&mut c, &[op]), 4, "PUSH {}", nombre);
        assert_eq!(c.stack_pointer, STACK - 2, "PUSH {}", nombre);
        assert_eq!(c.memory.read_byte(STACK - 1), 0x12, "byte alto de {}", nombre);
        assert_eq!(c.memory.read_byte(STACK - 2), 0x34, "byte bajo de {}", nombre);
    }

    // PUSH AF
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0xAB;
    c.work_registers[R8::F as usize] = 0xC0;
    exec(&mut c, &[0xF5]);
    assert_eq!(c.memory.read_byte(STACK - 1), 0xAB, "PUSH AF: byte alto es A");
    assert_eq!(c.memory.read_byte(STACK - 2), 0xC0, "PUSH AF: byte bajo es F");
}

#[test]
fn pop_r16()
{
    for (op, pair, nombre) in [(0xC1u8, R16::BC as u8, "BC"),
                               (0xD1,   R16::DE as u8, "DE"),
                               (0xE1,   R16::HL as u8, "HL")]
    {
        let mut c = cpu();
        c.stack_pointer = STACK;
        c.memory.write_byte(STACK, 0x34);
        c.memory.write_byte(STACK + 1, 0x12);
        assert_eq!(exec(&mut c, &[op]), 3, "POP {}", nombre);
        assert_eq!(c.get_r16(pair), 0x1234, "POP {}", nombre);
        assert_eq!(c.stack_pointer, STACK + 2, "POP {}", nombre);
    }

    // POP AF: el nibble bajo de F siempre queda a cero
    let mut c = cpu();
    c.stack_pointer = STACK;
    c.memory.write_byte(STACK, 0x3F);
    c.memory.write_byte(STACK + 1, 0xAB);
    exec(&mut c, &[0xF1]);
    assert_eq!(c.work_registers[R8::A as usize], 0xAB, "POP AF");
    assert_eq!(f(&c), 0x30, "POP AF debe descartar el nibble bajo de F");
}

#[test]
fn push_pop_ida_y_vuelta()
{
    let mut c = cpu();
    c.set_r16(R16::BC as u8, 0xBEEF);
    exec(&mut c, &[0xC5]);       // PUSH BC
    c.set_r16(R16::BC as u8, 0);
    exec(&mut c, &[0xC1]);       // POP BC
    assert_eq!(c.get_r16(R16::BC as u8), 0xBEEF);
    assert_eq!(c.stack_pointer, STACK, "la pila debe quedar equilibrada");
}

#[test]
fn call_imm16()
{
    let mut c = cpu();
    assert_eq!(exec(&mut c, &[0xCD, 0x00, 0xC6]), 6, "CALL imm16");
    assert_eq!(c.program_counter, 0xC600, "CALL debe saltar al destino");
    assert_eq!(c.stack_pointer, STACK - 2);

    // La direccion de retorno es la instruccion siguiente (PROG + 3)
    let ret = (c.memory.read_byte(STACK - 1) as usize) << 8
            | (c.memory.read_byte(STACK - 2) as usize);
    assert_eq!(ret, PROG + 3, "CALL debe apilar la direccion de retorno");
}

#[test]
fn call_cond_imm16()
{
    let casos = [(0xC4u8, FZ, false), (0xCC, FZ, true), (0xD4, FC, false), (0xDC, FC, true)];

    for (op, flag, salta_con_flag) in casos
    {
        // Tomado
        let mut c = cpu();
        c.work_registers[R8::F as usize] = if salta_con_flag { flag } else { 0 };
        assert_eq!(exec(&mut c, &[op, 0x00, 0xC6]), 6, "CALL cond {:02X} tomado", op);
        assert_eq!(c.program_counter, 0xC600);
        assert_eq!(c.stack_pointer, STACK - 2);

        // No tomado
        let mut c = cpu();
        c.work_registers[R8::F as usize] = if salta_con_flag { 0 } else { flag };
        assert_eq!(exec(&mut c, &[op, 0x00, 0xC6]), 3, "CALL cond {:02X} no tomado", op);
        assert_eq!(c.program_counter, PROG + 3);
        assert_eq!(c.stack_pointer, STACK, "un CALL no tomado no toca la pila");
    }
}

#[test]
fn ret()
{
    let mut c = cpu();
    c.stack_pointer = STACK;
    c.memory.write_byte(STACK, 0x00);
    c.memory.write_byte(STACK + 1, 0xC6);
    assert_eq!(exec(&mut c, &[0xC9]), 4, "RET");
    assert_eq!(c.program_counter, 0xC600);
    assert_eq!(c.stack_pointer, STACK + 2);
}

#[test]
fn reti()
{
    let mut c = cpu();
    c.ime = false;
    c.stack_pointer = STACK;
    c.memory.write_byte(STACK, 0x00);
    c.memory.write_byte(STACK + 1, 0xC6);
    exec(&mut c, &[0xD9]);
    assert_eq!(c.program_counter, 0xC600);
    assert!(c.ime, "RETI debe reactivar las interrupciones");
}

#[test]
fn ret_cond()
{
    let casos = [(0xC0u8, FZ, false), (0xC8, FZ, true), (0xD0, FC, false), (0xD8, FC, true)];

    for (op, flag, retorna_con_flag) in casos
    {
        // Tomado
        let mut c = cpu();
        c.stack_pointer = STACK;
        c.memory.write_byte(STACK, 0x00);
        c.memory.write_byte(STACK + 1, 0xC6);
        c.work_registers[R8::F as usize] = if retorna_con_flag { flag } else { 0 };
        assert_eq!(exec(&mut c, &[op]), 5, "RET cond {:02X} tomado", op);
        assert_eq!(c.program_counter, 0xC600);

        // No tomado
        let mut c = cpu();
        c.work_registers[R8::F as usize] = if retorna_con_flag { 0 } else { flag };
        assert_eq!(exec(&mut c, &[op]), 2, "RET cond {:02X} no tomado", op);
        assert_eq!(c.program_counter, PROG + 1);
        assert_eq!(c.stack_pointer, STACK, "un RET no tomado no toca la pila");
    }
}

#[test]
fn call_y_ret_juntos()
{
    let mut c = cpu();
    exec(&mut c, &[0xCD, 0x00, 0xC6]); // CALL 0xC600
    c.memory.write_byte(0xC600, 0xC9); // RET
    c.program_counter = 0xC600;
    c.step();
    assert_eq!(c.program_counter, PROG + 3, "RET debe volver justo detras del CALL");
    assert_eq!(c.stack_pointer, STACK);
}

#[test]
fn jp_imm16()
{
    let mut c = cpu();
    assert_eq!(exec(&mut c, &[0xC3, 0x34, 0x12]), 4, "JP imm16");
    assert_eq!(c.program_counter, 0x1234);
}

#[test]
fn jp_cond_imm16()
{
    let casos = [(0xC2u8, FZ, false), (0xCA, FZ, true), (0xD2, FC, false), (0xDA, FC, true)];

    for (op, flag, salta_con_flag) in casos
    {
        let mut c = cpu();
        c.work_registers[R8::F as usize] = if salta_con_flag { flag } else { 0 };
        assert_eq!(exec(&mut c, &[op, 0x34, 0x12]), 4, "JP cond {:02X} tomado", op);
        assert_eq!(c.program_counter, 0x1234);

        let mut c = cpu();
        c.work_registers[R8::F as usize] = if salta_con_flag { 0 } else { flag };
        assert_eq!(exec(&mut c, &[op, 0x34, 0x12]), 3, "JP cond {:02X} no tomado", op);
        assert_eq!(c.program_counter, PROG + 3);
    }
}

#[test]
fn jp_hl()
{
    let mut c = cpu();
    c.set_r16(R16::HL as u8, 0x1234);
    assert_eq!(exec(&mut c, &[0xE9]), 1, "JP HL");
    assert_eq!(c.program_counter, 0x1234);
}

#[test]
fn rst_todos_los_vectores()
{
    for n in 0..8u8
    {
        let mut c = cpu();
        let op = 0b11_000_111 | (n << 3);
        assert_eq!(exec(&mut c, &[op]), 4, "RST {:02X}", n * 8);
        assert_eq!(c.program_counter, (n as usize) * 8, "RST {:02X}", n * 8);

        let ret = (c.memory.read_byte(STACK - 1) as usize) << 8
                | (c.memory.read_byte(STACK - 2) as usize);
        assert_eq!(ret, PROG + 1, "RST debe apilar la direccion siguiente");
    }
}

#[test]
fn ldh_imm8()
{
    // LD [FF00+n], A
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x5A;
    assert_eq!(exec(&mut c, &[0xE0, 0x80]), 3, "LD [FF00+n], A");
    assert_eq!(c.memory.read_byte(0xFF80), 0x5A);
    assert_eq!(c.program_counter, PROG + 2);

    // LD A, [FF00+n]
    let mut c = cpu();
    c.memory.write_byte(0xFF80, 0xA5);
    assert_eq!(exec(&mut c, &[0xF0, 0x80]), 3, "LD A, [FF00+n]");
    assert_eq!(c.work_registers[R8::A as usize], 0xA5);
}

#[test]
fn ldh_c()
{
    // LD [FF00+C], A
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x5A;
    c.work_registers[RC as usize] = 0x81;
    assert_eq!(exec(&mut c, &[0xE2]), 2, "LD [FF00+C], A");
    assert_eq!(c.memory.read_byte(0xFF81), 0x5A);
    assert_eq!(c.program_counter, PROG + 1);

    // LD A, [FF00+C]
    let mut c = cpu();
    c.work_registers[RC as usize] = 0x81;
    c.memory.write_byte(0xFF81, 0xA5);
    assert_eq!(exec(&mut c, &[0xF2]), 2, "LD A, [FF00+C]");
    assert_eq!(c.work_registers[R8::A as usize], 0xA5);
}

#[test]
fn ld_imm16_a_y_vuelta()
{
    let mut c = cpu();
    c.work_registers[R8::A as usize] = 0x7C;
    assert_eq!(exec(&mut c, &[0xEA, (DATA & 0xFF) as u8, (DATA >> 8) as u8]), 4, "LD [imm16], A");
    assert_eq!(c.memory.read_byte(DATA), 0x7C);
    assert_eq!(c.program_counter, PROG + 3);

    let mut c = cpu();
    c.memory.write_byte(DATA, 0xC7);
    assert_eq!(exec(&mut c, &[0xFA, (DATA & 0xFF) as u8, (DATA >> 8) as u8]), 4, "LD A, [imm16]");
    assert_eq!(c.work_registers[R8::A as usize], 0xC7);
}

#[test]
fn add_sp_imm8()
{
    // Half-carry sobre el nibble bajo de SP
    let mut c = cpu();
    c.stack_pointer = 0x000F;
    assert_eq!(exec(&mut c, &[0xE8, 0x01]), 4, "ADD SP, imm8");
    assert_eq!(c.stack_pointer, 0x0010);
    assert_flags(&c, false, false, true, false, "ADD SP con half-carry");

    // Carry sobre el byte bajo
    let mut c = cpu();
    c.stack_pointer = 0x00FF;
    exec(&mut c, &[0xE8, 0x01]);
    assert_eq!(c.stack_pointer, 0x0100);
    assert_flags(&c, false, false, true, true, "ADD SP con carry");

    // Offset negativo
    let mut c = cpu();
    c.stack_pointer = 0x0100;
    exec(&mut c, &[0xE8, 0xFF]); // -1
    assert_eq!(c.stack_pointer, 0x00FF, "ADD SP con offset negativo");
    assert_eq!(c.program_counter, PROG + 2);
}

#[test]
fn ld_hl_sp_imm8()
{
    let mut c = cpu();
    c.stack_pointer = 0x000F;
    assert_eq!(exec(&mut c, &[0xF8, 0x01]), 3, "LD HL, SP+imm8");
    assert_eq!(c.get_r16(R16::HL as u8), 0x0010);
    assert_eq!(c.stack_pointer, 0x000F, "LD HL, SP+e8 no debe modificar SP");
    assert_flags(&c, false, false, true, false, "LD HL, SP+e8");
}

#[test]
fn ld_sp_hl()
{
    let mut c = cpu();
    c.set_r16(R16::HL as u8, 0x1234);
    assert_eq!(exec(&mut c, &[0xF9]), 2, "LD SP, HL");
    assert_eq!(c.stack_pointer, 0x1234);
    assert_eq!(c.program_counter, PROG + 1);
}

#[test]
fn di_ei()
{
    let mut c = cpu();
    c.ime = true;
    assert_eq!(exec(&mut c, &[0xF3]), 1, "DI");
    assert!(!c.ime);
    assert_eq!(c.program_counter, PROG + 1);

    let mut c = cpu();
    c.ime = false;
    assert_eq!(exec(&mut c, &[0xFB]), 1, "EI");
    assert!(c.ime);
}

#[test]
fn opcodes_invalidos_no_hacen_panic()
{
    // Estos huecos no existen en el hardware; lo unico que se exige es no reventar
    for op in [0xD3u8, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD]
    {
        let mut c = cpu();
        exec(&mut c, &[op]);
    }
}


// ---------------------------------------------------------------------------
// Prefijo 0xCB
// ---------------------------------------------------------------------------

#[test]
fn cb_rlc()
{
    for r in REGS
    {
        let mut c = cpu();
        c.work_registers[r as usize] = 0b1000_0001;
        assert_eq!(exec(&mut c, &[0xCB, 0b00_000_000 | r]), 2, "RLC r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0b0000_0011, "RLC r8 (indice {})", r);
        assert_flags(&c, false, false, false, true, "RLC");
        assert_eq!(c.program_counter, PROG + 2);
    }

    let mut c = cpu();
    set_hl_mem(&mut c, 0b1000_0001);
    assert_eq!(exec(&mut c, &[0xCB, 0x06]), 4, "RLC [HL]");
    assert_eq!(c.memory.read_byte(DATA), 0b0000_0011);
}

#[test]
fn cb_rrc()
{
    for r in REGS
    {
        let mut c = cpu();
        c.work_registers[r as usize] = 0b0000_0011;
        assert_eq!(exec(&mut c, &[0xCB, 0b00_001_000 | r]), 2, "RRC r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0b1000_0001);
        assert_flags(&c, false, false, false, true, "RRC");
    }

    let mut c = cpu();
    set_hl_mem(&mut c, 0b0000_0011);
    assert_eq!(exec(&mut c, &[0xCB, 0x0E]), 4, "RRC [HL]");
    assert_eq!(c.memory.read_byte(DATA), 0b1000_0001);
}

#[test]
fn cb_rl()
{
    for r in REGS
    {
        let mut c = cpu();
        c.work_registers[r as usize] = 0b1000_0000;
        c.work_registers[R8::F as usize] = FC;
        assert_eq!(exec(&mut c, &[0xCB, 0b00_010_000 | r]), 2, "RL r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0b0000_0001, "RL debe meter el carry viejo");
        assert_flags(&c, false, false, false, true, "RL");
    }

    // Resultado cero pone Z
    let mut c = cpu();
    c.work_registers[RB as usize] = 0b1000_0000;
    exec(&mut c, &[0xCB, 0x10]);
    assert_eq!(c.work_registers[RB as usize], 0x00);
    assert_flags(&c, true, false, false, true, "RL con resultado cero");

    let mut c = cpu();
    set_hl_mem(&mut c, 0b1000_0000);
    assert_eq!(exec(&mut c, &[0xCB, 0x16]), 4, "RL [HL]");
}

#[test]
fn cb_rr()
{
    for r in REGS
    {
        let mut c = cpu();
        c.work_registers[r as usize] = 0b0000_0001;
        c.work_registers[R8::F as usize] = FC;
        assert_eq!(exec(&mut c, &[0xCB, 0b00_011_000 | r]), 2, "RR r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0b1000_0000, "RR debe meter el carry viejo");
        assert_flags(&c, false, false, false, true, "RR");
    }

    let mut c = cpu();
    set_hl_mem(&mut c, 0b0000_0001);
    assert_eq!(exec(&mut c, &[0xCB, 0x1E]), 4, "RR [HL]");
}

#[test]
fn cb_sla()
{
    for r in REGS
    {
        let mut c = cpu();
        c.work_registers[r as usize] = 0b1000_0001;
        assert_eq!(exec(&mut c, &[0xCB, 0b00_100_000 | r]), 2, "SLA r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0b0000_0010, "SLA mete un 0 por la derecha");
        assert_flags(&c, false, false, false, true, "SLA");
    }

    let mut c = cpu();
    set_hl_mem(&mut c, 0b1000_0001);
    assert_eq!(exec(&mut c, &[0xCB, 0x26]), 4, "SLA [HL]");
}

#[test]
fn cb_sra()
{
    for r in REGS
    {
        // El bit 7 se conserva (desplazamiento aritmetico)
        let mut c = cpu();
        c.work_registers[r as usize] = 0b1000_0001;
        assert_eq!(exec(&mut c, &[0xCB, 0b00_101_000 | r]), 2, "SRA r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0b1100_0000, "SRA conserva el bit 7");
        assert_flags(&c, false, false, false, true, "SRA");
    }

    let mut c = cpu();
    set_hl_mem(&mut c, 0b1000_0001);
    assert_eq!(exec(&mut c, &[0xCB, 0x2E]), 4, "SRA [HL]");
}

#[test]
fn cb_swap()
{
    for r in REGS
    {
        let mut c = cpu();
        c.work_registers[r as usize] = 0xAB;
        c.work_registers[R8::F as usize] = FC;
        assert_eq!(exec(&mut c, &[0xCB, 0b00_110_000 | r]), 2, "SWAP r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0xBA, "SWAP intercambia los nibbles");
        assert_flags(&c, false, false, false, false, "SWAP debe limpiar C");
    }

    let mut c = cpu();
    c.work_registers[RB as usize] = 0x00;
    exec(&mut c, &[0xCB, 0x30]);
    assert_flags(&c, true, false, false, false, "SWAP con resultado cero");

    let mut c = cpu();
    set_hl_mem(&mut c, 0xAB);
    assert_eq!(exec(&mut c, &[0xCB, 0x36]), 4, "SWAP [HL]");
    assert_eq!(c.memory.read_byte(DATA), 0xBA);
}

#[test]
fn cb_srl()
{
    for r in REGS
    {
        // A diferencia de SRA, el bit 7 se pierde
        let mut c = cpu();
        c.work_registers[r as usize] = 0b1000_0001;
        assert_eq!(exec(&mut c, &[0xCB, 0b00_111_000 | r]), 2, "SRL r8 (indice {})", r);
        assert_eq!(c.work_registers[r as usize], 0b0100_0000, "SRL mete un 0 por la izquierda");
        assert_flags(&c, false, false, false, true, "SRL");
    }

    let mut c = cpu();
    set_hl_mem(&mut c, 0b1000_0001);
    assert_eq!(exec(&mut c, &[0xCB, 0x3E]), 4, "SRL [HL]");
}

#[test]
fn cb_bit_todos_los_bits_y_registros()
{
    for bit in 0..8u8
    {
        for r in REGS
        {
            let op = 0b01_000_000 | (bit << 3) | r;

            // Bit puesto -> Z=0
            let mut c = cpu();
            c.work_registers[r as usize] = 1 << bit;
            c.work_registers[R8::F as usize] = FC; // BIT no toca C
            assert_eq!(exec(&mut c, &[0xCB, op]), 2, "BIT {}, r8 (indice {})", bit, r);
            assert_flags(&c, false, false, true, true, "BIT con el bit puesto");
            assert_eq!(c.program_counter, PROG + 2);

            // Bit a cero -> Z=1
            let mut c = cpu();
            c.work_registers[r as usize] = !(1 << bit);
            assert_eq!(exec(&mut c, &[0xCB, op]), 2, "BIT {}, r8 (indice {})", bit, r);
            assert_flags(&c, true, false, true, false, "BIT con el bit a cero");
        }

        // BIT n, [HL]
        let mut c = cpu();
        set_hl_mem(&mut c, 1 << bit);
        assert_eq!(exec(&mut c, &[0xCB, 0b01_000_110 | (bit << 3)]), 3, "BIT {}, [HL]", bit);
        assert_flags(&c, false, false, true, false, "BIT n, [HL]");
    }
}

#[test]
fn cb_res_todos_los_bits_y_registros()
{
    for bit in 0..8u8
    {
        for r in REGS
        {
            let mut c = cpu();
            c.work_registers[r as usize] = 0xFF;
            c.work_registers[R8::F as usize] = 0xF0;

            let op = 0b10_000_000 | (bit << 3) | r;
            assert_eq!(exec(&mut c, &[0xCB, op]), 2, "RES {}, r8 (indice {})", bit, r);
            assert_eq!(c.work_registers[r as usize], !(1u8 << bit),
                "RES {}, r8 (indice {})", bit, r);
            assert_eq!(f(&c), 0xF0, "RES no debe tocar los flags");
        }

        let mut c = cpu();
        set_hl_mem(&mut c, 0xFF);
        assert_eq!(exec(&mut c, &[0xCB, 0b10_000_110 | (bit << 3)]), 4, "RES {}, [HL]", bit);
        assert_eq!(c.memory.read_byte(DATA), !(1u8 << bit));
    }
}

#[test]
fn cb_set_todos_los_bits_y_registros()
{
    for bit in 0..8u8
    {
        for r in REGS
        {
            let mut c = cpu();
            c.work_registers[r as usize] = 0x00;
            c.work_registers[R8::F as usize] = 0xF0;

            let op = 0b11_000_000 | (bit << 3) | r;
            assert_eq!(exec(&mut c, &[0xCB, op]), 2, "SET {}, r8 (indice {})", bit, r);
            assert_eq!(c.work_registers[r as usize], 1u8 << bit,
                "SET {}, r8 (indice {})", bit, r);
            assert_eq!(f(&c), 0xF0, "SET no debe tocar los flags");
        }

        let mut c = cpu();
        set_hl_mem(&mut c, 0x00);
        assert_eq!(exec(&mut c, &[0xCB, 0b11_000_110 | (bit << 3)]), 4, "SET {}, [HL]", bit);
        assert_eq!(c.memory.read_byte(DATA), 1u8 << bit);
    }
}


// ---------------------------------------------------------------------------
// Interrupciones
// ---------------------------------------------------------------------------

#[test]
fn handle_interrupts_salta_al_vector()
{
    // (bit de IF, vector)
    let vectores = [(0u8, 0x40usize), (1, 0x48), (2, 0x50), (3, 0x58), (4, 0x60)];

    for (bit, vector) in vectores
    {
        let mut c = cpu();
        c.ime = true;
        c.program_counter = 0xC600;
        c.memory.write_byte(0xFF0F, 1 << bit);
        c.memory.write_byte(0xFFFF, 1 << bit);

        assert_eq!(c.handle_interrupts(), 20, "interrupcion {}", bit);
        assert_eq!(c.program_counter, vector, "interrupcion {}", bit);
        assert!(!c.ime, "atender una interrupcion desactiva IME");
        assert_eq!(c.memory.read_byte(0xFF0F) & (1 << bit), 0,
            "el bit atendido debe limpiarse en IF");

        let ret = (c.memory.read_byte(STACK - 1) as usize) << 8
                | (c.memory.read_byte(STACK - 2) as usize);
        assert_eq!(ret, 0xC600, "debe apilarse el PC de retorno");
    }
}

#[test]
fn handle_interrupts_respeta_ime_y_el_registro_ie()
{
    // IME apagado: no se atiende
    let mut c = cpu();
    c.ime = false;
    c.memory.write_byte(0xFF0F, 0x01);
    c.memory.write_byte(0xFFFF, 0x01);
    assert_eq!(c.handle_interrupts(), 0);
    assert_eq!(c.program_counter, PROG - PROG); // sigue en 0

    // Pendiente pero no habilitada en IE: no se atiende
    let mut c = cpu();
    c.ime = true;
    c.memory.write_byte(0xFF0F, 0x01);
    c.memory.write_byte(0xFFFF, 0x00);
    assert_eq!(c.handle_interrupts(), 0);
    assert!(c.ime, "IME sigue activo si no se atiende nada");
}

#[test]
fn una_interrupcion_pendiente_despierta_del_halt()
{
    // Aunque IME este apagado, una interrupcion pendiente saca del estado halted
    let mut c = cpu();
    c.halted = true;
    c.ime = false;
    c.memory.write_byte(0xFF0F, 0x04);
    c.memory.write_byte(0xFFFF, 0x04);
    c.handle_interrupts();
    assert!(!c.halted, "una interrupcion pendiente despierta la CPU aunque IME=0");
}

#[test]
fn la_prioridad_es_del_bit_mas_bajo()
{
    let mut c = cpu();
    c.ime = true;
    c.program_counter = 0xC600;
    c.memory.write_byte(0xFF0F, 0b0001_0101); // V-Blank, Timer y Joypad a la vez
    c.memory.write_byte(0xFFFF, 0xFF);

    c.handle_interrupts();
    assert_eq!(c.program_counter, 0x40, "V-Blank tiene la prioridad mas alta");
    assert_eq!(c.memory.read_byte(0xFF0F), 0b0001_0100, "solo se limpia el bit atendido");
}
