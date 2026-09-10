use std::fs;
use std::thread;
use std::time::{Duration, Instant};
use std::hint::spin_loop;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

mod memory;
mod timer;
mod cpu;
mod ppu;
mod cartrige;

const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;
const FRAME_DURATION: Duration = Duration::from_nanos(16_742_706);
const TICK_PER_FRAME: u32 = 17556;

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

    let path: &str = "path";
    let romdata = fs::read(path)
        .expect("No se pudo abrir el archivo");
    let mut emulator = Emulator::new(romdata);


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

        let ant_joypad_register = emulator.cpu.memory.address_bus[0xFF00];

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
            
            // emulator.apu.step(ticks_gastados);
                        
            emulator.read_joypad();
            ticks_frame -= used_ticks as u32;
        
        }

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


/*
Event::AboutToWait =>
            {
                let now = std::time::Instant::now();

                if now >= next_frame
                {
                    
                    let ant_joypad_register = emulator.cpu.raw_memory.address_bus[0xFF00];

                    emulator.read_joypad();

                    if ant_joypad_register != emulator.cpu.raw_memory.address_bus[0xFF00]
                    {
                        
                        println!("joypad_register: {:08b}", emulator.cpu.raw_memory.address_bus[0xFF00]);
                        
                    }

                    let mut ticks_frame = 17556 as u16;
                    
                    while ticks_frame > 0
                    {
                        
                        let mut ticks_gastados = emulator.cpu.handle_interrupts() as u16; 

                        if ticks_gastados == 0
                        {
                            if !emulator.cpu.halted
                            {

                                ticks_gastados = emulator.cpu.step() as u16;
                            
                            }
                            
                        }

                        emulator.ppu.step(ticks_gastados, &mut emulator.cpu.raw_memory);
                        emulator.timer.step(ticks_gastados as u8, &mut emulator.cpu.raw_memory);
                        // emulator.apu.step(ticks_gastados);
                        
                        emulator.read_joypad();
                        ticks_frame -= ticks_gastados as u16;

                    }
                     

                    // Aqui ira la logica de CPU: emulator.cpu.step(), emulator.ppu.step(), etc.


                    

                    window.request_redraw();

                    next_frame = now + frame_duration;
                    event_loop_target.set_control_flow(ControlFlow::WaitUntil(next_frame));
                }
            }
*/
