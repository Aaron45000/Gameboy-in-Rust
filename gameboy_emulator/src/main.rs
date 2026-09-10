use std::fs;
use std::fs::File;
use std::thread;
use std::time::{Duration, Instant};
use std::hint::spin_loop;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};

mod memory;
mod timer;
mod cpu;
mod ppu;
mod cartrige;

const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;
const FRAME_DURATION: Duration = Duration::from_nanos(16_742_706);
const TICK_PER_FRAME: u32 = 17556;

// Paleta de grises: color-id 0..3 -> (R, G, B)
const SHADES: [(u8, u8, u8); 4] = [
    (255, 255, 255),
    (192, 192, 192),
    (96, 96, 96),
    (0, 0, 0),
];

// ---------------------------------------------------------------------------
// Joypad
// ---------------------------------------------------------------------------

struct Joypad 
{
    right: bool,
    left: bool,
    up: bool,
    down: bool,
    a: bool,
    b: bool,
    select: bool,
    start: bool,
}

impl Joypad
{
    fn new() -> Self
    {
        return Joypad 
        {
            right: true,
            left: true,
            up: true,
            down: true,
            a: true,
            b: true,
            select: true,
            start: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Emulator
// ---------------------------------------------------------------------------

struct Emulator
{
    cpu: cpu::Cpu,
    joypad: Joypad,
    timer: timer::Timer,
    ppu: ppu::Ppu
}

impl Emulator
{
    fn new(romdata: Vec<u8>) -> Self
    {
        return Emulator
        {
            cpu: cpu::Cpu::new(romdata),
            joypad: Joypad::new(),
            timer: timer::Timer::new(),
            ppu: ppu::Ppu::new()
        }
    }

    fn read_joypad(&mut self)
    {
        let joypad_register: u8;
        
        let old_state = self.cpu.memory.read_byte(0xFF00);
        
        let p14 = (old_state >> 4) & 1; // Bit 4: 0 = joypad
        let p15 = (old_state >> 5) & 1; // Bit 5: 0 = AB Start Select

        if p15 == 0
        {
            if p14 == 0
            {
                // Ambos seleccionados (raro pero posible)
                joypad_register = ((self.joypad.a && self.joypad.right) as u8)
                                | ((self.joypad.b && self.joypad.left) as u8) << 1
                                | ((self.joypad.select && self.joypad.up) as u8) << 2
                                | ((self.joypad.start && self.joypad.down) as u8) << 3;
            }
            else 
            {
                // Bit 5 = 0, Bit 4 = 1: leer botones de accion (A, B, Select, Start)
                joypad_register = (self.joypad.a as u8)
                                | (self.joypad.b as u8) << 1
                                | (self.joypad.select as u8) << 2
                                | (self.joypad.start as u8) << 3;
            }
        }
        else if p14 == 0
        {
            // Bit 5 = 1, Bit 4 = 0: leer direcciones (Right, Left, Up, Down)
            joypad_register = (self.joypad.right as u8)
                            | (self.joypad.left as u8) << 1
                            | (self.joypad.up as u8) << 2
                            | (self.joypad.down as u8) << 3;
        }
        else 
        {
            // Ninguno seleccionado
            joypad_register = 0x0F;
        }

        let new_state = (old_state & 0xF0) | joypad_register;
        
        self.cpu.memory.write_byte(0xFF00, new_state);

        if (old_state & !new_state & 0x0F) != 0 // interrupcion general
        {
            
            let current_if = self.cpu.memory.read_byte(0xFF0F);
            self.cpu.memory.write_byte(0xFF0F, current_if | 0b0001_0000); 

        }

    }

}



fn main()
{

    let path: &str = "path"; //placeholder
    let romdata = fs::read(path)
        .expect("No se pudo abrir el archivo");
    let mut emulator = Emulator::new(romdata);

    let save = fs::read("save.sav");

    if !save.is_err()
    {

        // Si save existe entonces lo carga a la ram del cartucho
        emulator.cpu.memory.cartrige.load_ram(save.unwrap());

    }

    // Inicializa el contexto principal de la biblioteca SDL2
    
    let sdl_context = sdl2::init().unwrap();

    // Inicializa específicamente el subsistema SDL2 
    let video_subsystem = sdl_context.video().unwrap();

    // Crea y configura la ventana
    let window = video_subsystem
        .window("rust-sdl2 demo", 800, 600) 
        .position_centered()                
        .build()                            
        .unwrap();                          

    // Convierte la ventana en un Canvas
    let mut canvas = window.into_canvas().build().unwrap();

    // Textura donde se volcara el framebuffer de la PPU
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, SCREEN_WIDTH, SCREEN_HEIGHT)
        .unwrap();

    // Establece el color primario con el que dibujar
    canvas.set_draw_color(Color::RGB(0, 255, 255));

    // Limpia el lienzo con el color seteado
    canvas.clear();

    // Muestra en pantalla el contenido del lienzo post cambio
    canvas.present();

    // Crea el manager de eventos encargada de capturar entradas del usuario
    let mut event_pump = sdl_context.event_pump().unwrap();


    let mut next_frame = Instant::now();

    // Bucle principal de la aplicacion
    'running: loop 
    {

        next_frame += FRAME_DURATION;

        // Event handler.
        for event in event_pump.poll_iter() 
        {

            match event 
            {
                
                // Evento de cierre de la ventana (clic en la X)
                Event::Quit { .. } => 
                {

                    let save = emulator.cpu.memory.cartrige.save_ram();
                    fs::write("save.sav", &save).unwrap();
                    break 'running;
                }

                Event::KeyDown { keycode: Some(key), .. } => 
                {
                    match key 
                    {
                        Keycode::Escape => 
                        {

                            // cierra 

                        },
                        Keycode::W | Keycode::Up => emulator.joypad.up = false,
                        Keycode::A | Keycode::Left => emulator.joypad.left = false,
                        Keycode::S | Keycode::Down => emulator.joypad.down = false,
                        Keycode::D | Keycode::Right => emulator.joypad.right = false,
                        Keycode::K | Keycode::Z => emulator.joypad.a = false,
                        Keycode::L | Keycode::X => emulator.joypad.b = false,
                        Keycode::Return | Keycode::LShift => emulator.joypad.start = false,
                        Keycode::Backspace | Keycode::C => emulator.joypad.select = false,
                        _ => {}
                    }
                }
                
                // Tecla liberada (KeyUp)
                Event::KeyUp { keycode: Some(key), .. }=> 
                {
                    match key 
                    {
                        Keycode::W | Keycode::Up => emulator.joypad.up = true,
                        Keycode::A | Keycode::Left => emulator.joypad.left = true,
                        Keycode::S | Keycode::Down => emulator.joypad.down = true,
                        Keycode::D | Keycode::Right => emulator.joypad.right = true,
                        Keycode::K | Keycode::Z => emulator.joypad.a = true,
                        Keycode::L | Keycode::X => emulator.joypad.b = true,
                        Keycode::Return | Keycode::LShift => emulator.joypad.start = true,
                        Keycode::Backspace | Keycode::C => emulator.joypad.select = true,
                        _ => {}
                    }
                }
                
                _ => {}
                
            }

        }

        // --- Aquí iría la lógica adicional de tu juego/aplicación ---

        emulator.read_joypad();

        let mut ticks_frame = TICK_PER_FRAME;

        while ticks_frame > 0
        {

            let mut used_ticks = emulator.cpu.handle_interrupts() as u32; 

            if used_ticks == 0
            {
                if !emulator.cpu.halted
                {

                    used_ticks = emulator.cpu.step() as u32;
                            
                }
                else
                {
                    
                    used_ticks = 1;

                }
                            
            }

            emulator.ppu.step(used_ticks as u16, &mut emulator.cpu.memory);
            emulator.timer.step(used_ticks as u8, &mut emulator.cpu.memory);
            emulator.cpu.memory.cartrige.step(used_ticks as u16);
            // emulator.apu.step(ticks_gastados);
                        
            emulator.read_joypad();
            ticks_frame -= used_ticks as u32;
        
        }

        // La PPU devuelve color-ids (0..3); aqui se mapean a la paleta de
        // grises y se empaquetan en RGB24 (3 bytes por pixel) para la textura.
        let mut rgb = [0u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 3) as usize];
        {
            let pixels = emulator.ppu.get_pixels();
            for (i, &color_id) in pixels.iter().enumerate()
            {
                let (r, g, b) = SHADES[color_id as usize];
                rgb[i * 3] = r;
                rgb[i * 3 + 1] = g;
                rgb[i * 3 + 2] = b;
            }
        }

        // `copy` estira la textura de 160x144 al canvas completo y `present`
        // la muestra en pantalla.
        texture
            .update(None, &rgb, (SCREEN_WIDTH * 3) as usize)
            .unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        
        let now = Instant::now();

        if now < next_frame 
        {

            let remaining = next_frame - now;

            if remaining > Duration::from_millis(2) 
            {
                thread::sleep(remaining - Duration::from_millis(1));
            }
            while Instant::now() < next_frame
            {
                spin_loop();
            }
        }        

    }

}
