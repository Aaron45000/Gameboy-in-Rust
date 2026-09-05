use std::fs;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;
mod memory;
mod timer;
mod cpu;
mod ppu;
mod cartrige;


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
        
        // Leemos el estado anterior para detectar si algún botón acaba de ser presionado
        let old_state = self.cpu.raw_memory.read_byte(0xFF00);
        
        let p14 = (old_state >> 4) & 1; // Bit 4: 0 = Selecciona Direcciones
        let p15 = (old_state >> 5) & 1; // Bit 5: 0 = Selecciona Botones de acción

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
        
        // Escribimos el nuevo estado usando la función segura
        self.cpu.raw_memory.write_byte(0xFF00, new_state);

        // Si algún botón pasó de 1 a 0 (de soltado a presionado), disparamos la interrupción
        if (old_state & !new_state & 0x0F) != 0 
        {
            let current_if = self.cpu.raw_memory.read_byte(0xFF0F);
            // Encender bit 4 del registro IF (Interrupción de Joypad)
            self.cpu.raw_memory.write_byte(0xFF0F, current_if | 0b0001_0000); 
        }
    }
}




fn main() 
{
    let path: &str = "/home/aaron4500/Descargas/Pokemon - Edicion Roja (Spain) (SGB Enhanced).gb";
    let romdata = fs::read(path )
        .expect("No se pudo abrir el archivo");

    // El tipo de MBC y el numero de bancos los deduce la fabrica del cartucho
    let mut emulator = Emulator::new(romdata);

    
    emulator.cpu.raw_memory.address_bus[0xFF00] |= 0b11001111;

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("prueba inputs")
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(160, 144, surface_texture).unwrap();

    // Pintar fondo azul inicial
    for pixel in pixels.frame_mut().chunks_exact_mut(4) 
    {
        pixel[0] = 0x00; // R
        pixel[1] = 0x00; // G
        pixel[2] = 0xFF; // B
        pixel[3] = 0xFF; // A
    }
    pixels.render().unwrap();

    
    let frame_duration = std::time::Duration::from_secs_f64(1.0 / 59.7275);
    let mut next_frame = std::time::Instant::now() + frame_duration;

    // --- Bucle principal ---
    // Winit funciona con un sistema de eventos que rigen lo que pasa en la ventana, 
    // y se ejecuta en un bucle infinito hasta que se cierra la ventana
    event_loop.run(move |event, event_loop_target|
    {
        event_loop_target.set_control_flow(ControlFlow::WaitUntil(next_frame));

        match event
        {
            // Evento de cuando se pide que se cierre la ventana
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } =>
            {
                event_loop_target.exit();
            }

            // Evento de cuando se presiona o suelta una tecla del teclado
            Event::WindowEvent 
            { 
                event: WindowEvent::KeyboardInput 
                { 
                    event: KeyEvent { physical_key, state, .. }, .. 
                }, 
                .. 
            } =>
            {
                let presionado = state == ElementState::Pressed;

                match physical_key
                {
                    PhysicalKey::Code(KeyCode::KeyW)      => emulator.joypad.up     = !presionado,
                    PhysicalKey::Code(KeyCode::KeyA)      => emulator.joypad.left   = !presionado,
                    PhysicalKey::Code(KeyCode::KeyS)      => emulator.joypad.down   = !presionado,
                    PhysicalKey::Code(KeyCode::KeyD)      => emulator.joypad.right  = !presionado,
                    PhysicalKey::Code(KeyCode::KeyK)      => emulator.joypad.a      = !presionado,
                    PhysicalKey::Code(KeyCode::KeyL)      => emulator.joypad.b      = !presionado,
                    PhysicalKey::Code(KeyCode::Enter)     => emulator.joypad.start  = !presionado,
                    PhysicalKey::Code(KeyCode::Backspace) => emulator.joypad.select = !presionado,
                    _ => (),
                }
            }

            // Cuando ya no hay eventos pendientes: logica de frame
            // es un bucle cada x tiempo, ideal para dibujar y actualizar la logica del emulador 
            // frame por frame
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

            // Renderizar con pixels cuando la ventana lo pida
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } =>
            {
                const PALETTE: [[u8; 4]; 4] = [
                    [0xFF, 0xFF, 0xFF, 0xFF], // sombra 0: blanco
                    [0xAA, 0xAA, 0xAA, 0xFF], // sombra 1: gris claro
                    [0x55, 0x55, 0x55, 0xFF], // sombra 2: gris oscuro
                    [0x00, 0x00, 0x00, 0xFF], // sombra 3: negro
                ];

                let framebuffer = emulator.ppu.framebuffer();

                for (pixel, &shade) in pixels.frame_mut().chunks_exact_mut(4).zip(framebuffer.iter())
                {
                    pixel.copy_from_slice(&PALETTE[shade as usize]);
                }

                if pixels.render().is_err()
                {
                    event_loop_target.exit();
                }
            }

            _ => (),
        }
    }).unwrap();
}