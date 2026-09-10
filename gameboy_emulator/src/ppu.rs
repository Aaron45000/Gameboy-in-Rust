use crate::memory::Memory;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

enum PpuMode { HBlank = 0, VBlank = 1, OAMSearch = 2, PixelTransfer = 3 }

pub struct Ppu {
    ticks: u32,
    pixels: [u8; SCREEN_WIDTH * SCREEN_HEIGHT],
    bg_color_id: [u8; SCREEN_WIDTH],
    window_line: u8,
}

impl Ppu {
    pub fn new() -> Self
    {
        Ppu {
            ticks: 0,
            pixels: [0; SCREEN_WIDTH * SCREEN_HEIGHT],
            bg_color_id: [0; SCREEN_WIDTH],
            window_line: 0,
        }
    }

    /// Devuelve el framebuffer como color-ids (0..3), un byte por pixel.
    pub fn get_pixels(&self) -> &[u8]
    {
        &self.pixels
    }

    /* 
    Dibuja una linea completa del background.
    
     Flujo:
      1. Si LCDC bit 0 esta apagado, se rellena `bg_color_id` con 0 (fondo
         "blanco" a efectos de la prioridad de sprites).
      2. Se calcula la fila de tiles visible con el scroll vertical
         `y = (ly + SCY) & 0xFF`, que da el tile_row y el byte dentro del tile.
      3. Para cada uno de los 160 pixeles se calcula `x = (px + SCX) & 0xFF`,
         se lee el indice de tile del tilemap y se extrae el color-id de esa
         fila con `read_tile_row`.
      4. Por ultimo se aplica la paleta BGP (2 bits por color-id) y se escribe
         el resultado en el framebuffer.
    */

    fn render_background(&mut self, ly: u8, memory: &Memory)
    {
        let lcdc = memory.address_bus[0xFF40];
        let bgp = memory.address_bus[0xFF47];

        if lcdc & 0x01 == 0
        {
            // BG desactivado: todo se trata como color 0 (para la prioridad).
            for px in 0..SCREEN_WIDTH
            {
                self.bg_color_id[px] = 0;
            }
        }
        else
        {
            let scx = memory.address_bus[0xFF43] as usize;
            let scy = memory.address_bus[0xFF42] as usize;
            // LCDC bit 3 elige entre los dos tilemaps (0x9800 o 0x9C00).
            let tilemap_base: u16 = if lcdc & 0x08 == 0 { 0x9800 } else { 0x9C00 };

            // Posicion en el mapa (256x256) teniendo en cuenta el scroll vertical.
            let y = (ly as usize + scy) & 0xFF;
            let tile_row = y / 8;
            let y_in_tile = (y % 8) as u8;

            for px in 0..SCREEN_WIDTH
            {
                // Posicion en el mapa con el scroll horizontal.
                let x = (px + scx) & 0xFF;
                let tile_col = x / 8;
                let x_in_tile = x % 8;

                // Cada tilemap es de 32x32 tiles: offset = fila*32 + columna.
                let tile_index = memory.read_byte(tilemap_base + (tile_row * 32 + tile_col) as u16);
                let tile_addr = get_tile_data_address(lcdc, tile_index, y_in_tile);
                let row = read_tile_row(memory, tile_addr);
                self.bg_color_id[px] = row[x_in_tile];
            }
        }

        // Aplica BGP y escribe la linea en el framebuffer.
        let line = ly as usize * SCREEN_WIDTH;
        for px in 0..SCREEN_WIDTH
        {
            let color_id = self.bg_color_id[px] as usize;
            self.pixels[line + px] = (bgp >> (color_id * 2)) & 0x03;
        }
    }

     /*
     Dibuja la linea de la ventana (Window), que se superpone al background.
    
     La ventana es un segundo tilemap "fijo" en pantalla (no se desplaza con
     SCX/SCY). Solo es visible cuando:
      - LCDC bit 5 esta activo.
      - La linea actual esta por debajo de WY (0xFF4A).
    
     Su posicion horizontal es `WX - 7` y su propio scroll vertical lo lleva
     el contador interno `window_line`, que avanza una vez por linea visible.
    */
    fn render_window(&mut self, ly: u8, memory: &Memory)
    {
        let lcdc = memory.address_bus[0xFF40];
        if lcdc & 0x20 == 0
        {
            return;
        }

        let wy = memory.address_bus[0xFF4A] as usize;
        if (ly as usize) < wy
        {
            return;
        }

        let bgp = memory.address_bus[0xFF47];
        // WX marca el borde izquierdo de la ventana + 7; restamos esos 7.
        let wx = memory.address_bus[0xFF4B] as i32 - 7;
        // LCDC bit 6 elige el tilemap de la ventana.
        let tilemap_base: u16 = if lcdc & 0x40 == 0 { 0x9800 } else { 0x9C00 };

        let window_y = self.window_line as usize;
        let tile_row = window_y / 8;
        let y_in_tile = (window_y % 8) as u8;

        for px in 0..SCREEN_WIDTH
        {
            // x relativo a la ventana; antes de WX no se dibuja nada.
            let x = px as i32 - wx;
            if x < 0
            {
                continue;
            }
            let x = x as usize;
            let tile_col = x / 8;
            let x_in_tile = x % 8;

            let tile_index = memory.read_byte(tilemap_base + (tile_row * 32 + tile_col) as u16);
            let tile_addr = get_tile_data_address(lcdc, tile_index, y_in_tile);
            let row = read_tile_row(memory, tile_addr);
            let color_id = row[x_in_tile] as usize;
            self.pixels[ly as usize * SCREEN_WIDTH + px] = (bgp >> (color_id * 2)) & 0x03;
        }

        self.window_line = self.window_line.wrapping_add(1);
    }

    /* 
    Dibuja los sprites (objetos) visibles en la linea actual.
    
     Cada sprite ocupa 4 bytes en OAM (0xFE00..0xFE9F): Y, X, tile, atributos.
     La Y guardada esta desplazada +16, asi que el borde real es `Y - 16`.
    
     Atributos:
      - bit 7: prioridad (1 = detras del BG)
      - bit 6: flip vertical
      - bit 5: flip horizontal
      - bit 4: paleta (0 = OBP0, 1 = OBP1)
    
     Se recorre OAM en orden inverso: en la Game Boy el sprite de menor indice
     tiene prioridad y debe quedar "encima", por eso se dibuja al final.
    */
    fn render_sprites(&mut self, ly: u8, memory: &Memory)
    {
        let lcdc = memory.address_bus[0xFF40];
        if lcdc & 0x02 == 0
        {
            return;
        }

        // LCDC bit 2: 0 = sprites de 8x8, 1 = de 8x16.
        let sprite_height: i32 = if lcdc & 0x04 == 0 { 8 } else { 16 };

        for i in (0..40).rev()
        {
            let oam = 0xFE00 + i * 4;
            let sprite_y = memory.read_byte(oam) as i32;
            let sprite_x = memory.read_byte(oam + 1) as i32;
            let tile_index = memory.read_byte(oam + 2);
            let attributes = memory.read_byte(oam + 3);

            // Rango vertical real del sprite en pantalla.
            let top = sprite_y - 16;
            let bottom = top + sprite_height;
            if (ly as i32) < top || (ly as i32) >= bottom
            {
                continue;
            }

            // Fila del sprite correspondiente a esta linea, con flip vertical.
            let mut y_in_sprite = ly as i32 - top;
            if attributes & 0x40 != 0
            {
                y_in_sprite = sprite_height - 1 - y_in_sprite;
            }

            // Los tiles de sprites siempre estan en el area unsigned (0x8000).
            let tile_addr = 0x8000u16 + (tile_index as u16) * 16 + (y_in_sprite as u16) * 2;
            let row = read_tile_row(memory, tile_addr);

            let x_flip = attributes & 0x20 != 0;
            let palette = if attributes & 0x10 != 0
            {
                memory.address_bus[0xFF49] // OBP1
            }
            else
            {
                memory.address_bus[0xFF48] // OBP0
            };
            let behind_bg = attributes & 0x80 != 0;

            for k in 0..8
            {
                let x = sprite_x - 8 + k as i32;
                if x < 0 || x >= SCREEN_WIDTH as i32
                {
                    continue;
                }
                let px = x as usize;

                let color_id = if x_flip { row[7 - k] } else { row[k] };
                if color_id == 0
                {
                    continue; // color 0 = transparente
                }

                // Si esta "detras del BG", solo se dibuja donde el BG es color 0.
                if behind_bg && self.bg_color_id[px] != 0
                {
                    continue;
                }

                self.pixels[ly as usize * SCREEN_WIDTH + px] = (palette >> (color_id as usize * 2)) & 0x03;
            }
        }
    }

    /* 
    Avanza la PPU `m_cycles` ciclos de maquina.
    
     Una linea completa dura 114 M-cycles (456 T-cycles) y se divide en:
      - Modo 2 (OAM Search):  ticks 0..19
      - Modo 3 (Pixel Transfer): ticks 20..62
      - Modo 0 (HBlank):      ticks 63..113
     Las lineas 144..153 forman el VBlank (modo 1).
    
     Ademas de actualizar el modo/LY, gestiona el flag LYC==LY y las
     interrupciones (VBlank y STAT). El render de una linea se dispara al
     entrar en HBlank, cuando VRAM y OAM vuelven a ser accesibles.
    */
    pub fn step(&mut self, m_cycles: u16, memory: &mut Memory)
    {
        self.ticks += m_cycles as u32;

        let lcdc = memory.address_bus[0xFF40];

        if lcdc & 0x80 == 0
        {
            // LCD apagado: reinicia LY, el framebuffer y el modo a HBlank.
            memory.address_bus[0xFF44] = 0;
            self.pixels = [0; SCREEN_WIDTH * SCREEN_HEIGHT];
            self.ticks = 0;
            self.window_line = 0;
            memory.ppu_mode = PpuMode::HBlank as u8;
            memory.address_bus[0xFF41] = (memory.address_bus[0xFF41] & 0xFC) | PpuMode::HBlank as u8;
            return;
        }

        let old_stat = memory.address_bus[0xFF41];
        let old_mode = old_stat & 0x03;
        let old_ly = memory.address_bus[0xFF44];
        let mut ly = old_ly;

        // Se ha completado una linea: avanza LY y, si toca, entra en VBlank.
        if self.ticks >= 114
        {
            self.ticks -= 114;
            ly = ly.wrapping_add(1);

            if ly > 153
            {
                ly = 0;
                self.window_line = 0;
            }

            if ly == 144
            {
                // Interrupcion de VBlank (bit 0 de IF).
                memory.address_bus[0xFF0F] |= 0x01;
            }

            memory.address_bus[0xFF44] = ly;
        }

        let current_mode = if ly >= 144
        {
            PpuMode::VBlank as u8
        }
        else if self.ticks < 20
        {
            PpuMode::OAMSearch as u8
        }
        else if self.ticks < 63
        {
            PpuMode::PixelTransfer as u8
        }
        else
        {
            PpuMode::HBlank as u8
        };

        let lyc = memory.address_bus[0xFF45];
        let coincidence = ly == lyc;

        // Escribe en STAT los bits de modo y el flag LYC==LY (bit 2).
        let mut stat = (old_stat & 0xFC) | current_mode;
        stat = (stat & !0x04) | ((coincidence as u8) << 2);
        memory.address_bus[0xFF41] = stat;
        // Actualiza el modo antes de renderizar para que VRAM/OAM no queden
        // bloqueadas por memory.rs durante el dibujado.
        memory.ppu_mode = current_mode;

        let mode_edge = current_mode != old_mode;

        // Al entrar en HBlank la linea ya esta transferida: se renderiza entera.
        if mode_edge && current_mode == PpuMode::HBlank as u8
        {
            self.render_background(ly, memory);
            self.render_window(ly, memory);
            self.render_sprites(ly, memory);
        }

        // Interrupcion STAT (0x48): se pide en los flancos de modo habilitados
        // (bits 3/4/5 de STAT) o al aparecer la coincidencia LYC==LY (bit 6).
        let mode_int = mode_edge
            && match current_mode
            {
                0 => stat & 0x08 != 0,
                1 => stat & 0x10 != 0,
                2 => stat & 0x20 != 0,
                _ => false,
            };
        let lyc_int = stat & 0x40 != 0 && coincidence && old_ly != lyc;

        if mode_int || lyc_int
        {
            let if_ = memory.read_byte(0xFF0F);
            memory.write_byte(0xFF0F, if_ | 0x02);
        }
    }
}

/* 
Devuelve la direccion de la fila `y_in_tile` del tile indicado.

 Hay dos modos segun LCDC bit 4:
  - Unsigned (bit 4 = 1): los indices van de 0..255 con base 0x8000.
  - Signed   (bit 4 = 0): el indice se interpreta como i8 con base 0x9000
    (permite acceder a los tiles "negativos" del bloque 0x8800..0x97FF).
*/
fn get_tile_data_address(lcdc: u8, tile_index: u8, y_in_tile: u8) -> u16
{
    if lcdc & 0x10 != 0
    {
        return 0x8000u16 + (tile_index as u16) * 16 + (y_in_tile as u16) * 2;
    }

    let base = 0x9000i32 + ((tile_index as i8) as i32) * 16;
    return base as u16 + (y_in_tile as u16) * 2;
}

 /*
 Lee una fila de 8 pixeles de un tile a partir de sus dos bytes.

 Cada tile de 8x8 se codifica en 16 bytes: 2 por fila. El primer byte es el
 bit "bajo" de cada pixel y el segundo el bit "alto"; juntos forman un
 color-id de 2 bits (0..3). Devuelve un array con esos 8 color-ids, de
 izquierda a derecha.
 */
fn read_tile_row(memory: &Memory, address: u16) -> [u8; 8]
{
    let lo = memory.read_byte(address);
    let hi = memory.read_byte(address + 1);
    let mut row = [0u8; 8];

    for i in 0..8
    {
        // bit 7 es el pixel mas a la izquierda.
        let bit = 7 - i;
        row[i] = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);
    }

    return row;
}
