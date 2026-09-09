# Gameboy in Rust

Un emulador de **Game Boy** (DMG) escrito en Rust desde cero, con fines educativos.

El proyecto emula el hardware del Game Boy clásico: la CPU (LR35902), la memoria, el temporizador, la PPU, el controlador (joypad) y los chips de memoria de los cartuchos (MBC).

> ⚠️ **Estado: en desarrollo.** Gran parte del núcleo ya funciona, pero la PPU (renderizado de gráficos) y el bucle de frames todavía están incompletos. Consulta la sección [Estado del proyecto](#estado-del-proyecto).

---

## Características

- **CPU (LR35902)** con el set de instrucciones completo:
  - Cargas de 8 y 16 bits (`ld r, r'`, `ld r, imm8`, `ld r16, imm16`, `ldh`, ...).
  - Operaciones ALU (`add`, `adc`, `sub`, `sbc`, `and`, `xor`, `or`, `cp`).
  - Aritmética de 16 bits (`add hl, r16`, `inc/dec r16`, `add sp, imm8`, ...).
  - Saltos y llamadas (`jp`, `jr`, `call`, `ret`, `rst`) con condiciones.
  - Instrucciones con prefijo `0xCB` (rotaciones, desplazamientos, bit/res/set).
  - Interrupciones, `HALT`, `EI`/`DI`, `DAA`, `STOP`, etc.
- **Memoria** completa de 64 KB con:
  - Bloqueo de VRAM y OAM durante los modos de la PPU.
  - Transferencias **DMA** (`0xFF46`).
  - Acceso a los registros de E/S mapeados en memoria.
- **Cartuchos** con varios tipos de MBC:
  - ROM only, **MBC1**, **MBC2**, **MBC3** (con RTC en tiempo real) y **MBC5**.
  - Detección automática del tipo de MBC, número de bancos y batería desde la cabecera de la ROM.
- **Temporizador** (`DIV`, `TIMA`, `TMA`, `TAC`) con interrupción de timer.
- **PPU** con los cuatro modos (OAM Search, Pixel Transfer, HBlank, VBlank) y el contador de líneas `LY`/`STAT`.
- **Joypad** mapeado al registro `0xFF00` con detección de pulsaciones e interrupción.
- **Tests unitarios** para la CPU y los cartuchos (90 tests).

---

## Estructura del proyecto

```
gameboy_emulator/
├── Cargo.toml
└── src/
    ├── main.rs          # Bucle de eventos (winit) y renderizado (softbuffer)
    ├── cpu.rs           # Núcleo de la CPU (fetch/decode/execute)
    │   └── cpu/         # Implementación de las instrucciones
    │       ├── alu.rs
    │       ├── arith16.rs
    │       ├── cb.rs
    │       ├── jumps.rs
    │       ├── loads.rs
    │       └── misc.rs
    ├── memory.rs        # Mapa de memoria y DMA
    ├── ppu.rs           # Procesador de imagen (modos y renderizado)
    ├── timer.rs         # Temporizador y divisor
    ├── cartrige.rs      # Detección y fabricación de cartuchos
    │   └── cartrige/    # Implementaciones de los MBC
    │       ├── header.rs
    │       ├── rom_only.rs
    │       ├── mbc1.rs
    │       ├── mbc2.rs
    │       ├── mbc3.rs
    │       └── mbc5.rs
    └── tests/           # Tests unitarios
        ├── cpu_tests.rs
        └── cartrige_tests.rs
```

---

## Requisitos

- [Rust](https://www.rust-lang.org/) (el proyecto usa la *edition 2024*, por lo que se recomienda una toolchain reciente).
- Un sistema con soporte para `winit`/`softbuffer` (Linux, macOS o Windows).

## Compilar y ejecutar

```bash
cd gameboy_emulator
cargo run
```

> Nota: la ruta de la ROM está hardcodeada como `"path"` en `src/main.rs:277`. Sustitúyela por la ruta a un archivo `.gb` válido.

## Ejecutar los tests

```bash
cd gameboy_emulator
cargo test
```

---

## Estado del proyecto

### Implementado
- Set de instrucciones de la CPU (con tests).
- Mapa de memoria, DMA y bloqueos de la PPU.
- MBC1, MBC2, MBC3 (con RTC) y MBC5.
- Temporizador completo.
- Modos de la PPU y contador de líneas (`LY`/`STAT`).

### Pendiente / TODO
- **Renderizado de la PPU**: `render_background`, `render_window` y `render_sprites` están vacíos (`src/ppu.rs`).
- **Dibujado del framebuffer** en la ventana (`RedrawRequested` en `src/main.rs`).
- **Bucle de frames** con el timing correcto (`about_to_wait` en `src/main.rs`).
- **Conexión del teclado** al joypad (los eventos de teclado aún no hacen nada).
- Persistencia de la RAM con batería (`save_ram`/`load_ram`) a disco.
- Algunos opcodes poco comunes aún no están cubiertos (devuelven `0` ciclos).

---

## Controles

| Tecla | Game Boy |
|-------|----------|
| `W` `A` `S` `D` | Cruceta (arriba/izquierda/abajo/derecha) |
| `K` `L` | A / B |
| `Enter` `Backspace` | Start / Select |

> Los controles están definidos pero todavía no están conectados al joypad.

---

## Dependencias

- [`winit`](https://crates.io/crates/winit) — gestión de ventanas y eventos.
- [`softbuffer`](https://crates.io/crates/softbuffer) — presentación del framebuffer en pantalla.

---

## Referencias útiles

- [Pan Docs](https://gbdev.io/pandocs/) — documentación técnica del Game Boy.
- [The Cycle-Accurate Game Boy Docs](https://github.com/AntonioND/giibiiadvance/tree/master/docs) — ciclos de la CPU.
- [gbdev/awesome-gbdev](https://github.com/gbdev/awesome-gbdev) — recursos sobre desarrollo para Game Boy.

---

## Licencia

Proyecto personal con fines educativos.
