use crate::memory::Memory;

enum PpuMode { HBlank = 0, VBlank = 1, OAMSearch = 2, PixelTransfer = 3 }

pub struct Ppu {
    ticks: u32,
    pixels: [u8; 160*144],
    bg_color_id: [u8; 160],
    window_line: u8,
}

impl Ppu {
    pub fn new() -> Self 
    {
        return Ppu { ticks: 0, pixels: [0; 160*144], bg_color_id: [0; 160], window_line: 0 }
    }


    fn render_background(&mut self, ly: u8, memory: &Memory) 
    {
        


    }
    
    fn render_window(&mut self, ly: u8, memory: &Memory) 
    {
        


    }

    fn render_sprites(&mut self, ly: u8, memory: &Memory) 
    {



    }

    pub fn step(&mut self, m_cycles: u16, memory: &mut Memory) 
    {
        self.ticks += m_cycles as u32;

        let mut ly = memory.address_bus[0xFF44];
        let mut stat = memory.address_bus[0xFF41];
        let old_mode = stat & 0x03; 
        let lcdc = memory.address_bus[0xFF40];
        let mut current_mode = old_mode;

        if (lcdc & 1<<7) == 0 {

            memory.address_bus[0xFF44] = 0;
            self.pixels = [0; 160*144];
            self.ticks = 0;
            self.window_line = 0;
            memory.ppu_mode = PpuMode::HBlank as u8;
            memory.address_bus[0xFF41] = (stat & !(0b11 as u8)) | PpuMode::HBlank as u8;
            return;
        }
        
        if self.ticks >= 114 {
            self.ticks -= 114;
            ly = ly.wrapping_add(1);

            if ly > 153 {
                ly = 0;
                self.window_line = 0;
            }


            if ly == 144 {
                memory.address_bus[0xFF0F] |= 0x01; 
            }
            
            memory.address_bus[0xFF44] = ly; 
        }

        
        if ly >= 144 
        {
            current_mode = PpuMode::VBlank as u8; 
        } 
        else 
        {
            
            if self.ticks <= 20 
            {
                current_mode = PpuMode::OAMSearch as u8;
            } 
            else if self.ticks <= 63 
            {
                current_mode = PpuMode::PixelTransfer as u8;
            } 
            else 
            {
                current_mode = PpuMode::HBlank as u8;
            }
        }

        
        if current_mode != old_mode 
        {
            
            if current_mode == PpuMode::HBlank as u8
            {
                self.render_background(ly, memory);
                self.render_window(ly, memory);
                self.render_sprites(ly, memory);
            }
            
        }

        stat = (stat & 0xFC) | current_mode;
        memory.address_bus[0xFF41] = stat;
        memory.ppu_mode = current_mode;
    }

}