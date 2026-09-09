use std::fs;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

mod memory;
mod timer;
mod cpu;
mod ppu;
mod cartrige;

const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;

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

enum AppState
{
    Uninitialized,
    Suspended { window: Rc<Window> },
    Running { surface: Surface<OwnedDisplayHandle, Rc<Window>> },
}

struct App
{
    context: Context<OwnedDisplayHandle>,
    state: AppState,
    emulator: Emulator,
    frame_duration: Duration,
    next_frame: Instant,
}

impl App
{
    fn new(context: Context<OwnedDisplayHandle>, emulator: Emulator) -> Self
    {
        return App
        {
            context,
            state: AppState::Uninitialized,
            emulator,
            frame_duration: Duration::from_secs_f64(1.0 / 59.7275),
            next_frame: Instant::now(),
        };
    }
}

impl ApplicationHandler for App
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop)
    {
        let window_attributes = Window::default_attributes()
            .with_title("Game Boy Emulator")
            .with_inner_size(LogicalSize::new(800, 720)); // 160x144 a escala x5

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

        let mut surface = Surface::new(&self.context, window.clone()).unwrap();
        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            surface.resize(width, height).unwrap();
        }

        self.state = AppState::Running { surface };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    )
    {
        let AppState::Running { surface } = &mut self.state else { return; };

        if surface.window().id() != window_id
        {
            return;
        }

        match event
        {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) =>
            {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    surface.resize(width, height).unwrap();
                }
            }

            WindowEvent::KeyboardInput
            {
                event: KeyEvent { physical_key, state, .. },
                ..
            } =>
            {
                let pressed = state == ElementState::Pressed;

                match physical_key
                {
                    PhysicalKey::Code(KeyCode::KeyW) => {},
                    PhysicalKey::Code(KeyCode::KeyA) => {},
                    PhysicalKey::Code(KeyCode::KeyS) => {},
                    PhysicalKey::Code(KeyCode::KeyD) => {},
                    PhysicalKey::Code(KeyCode::KeyK) => {},
                    PhysicalKey::Code(KeyCode::KeyL) => {},
                    PhysicalKey::Code(KeyCode::Enter) => {},
                    PhysicalKey::Code(KeyCode::Backspace) => {},
                    _ => {}
                }
                let _ = pressed;
            }

            WindowEvent::RedrawRequested =>
            {
                // TODO: renderizar el framebuffer de la PPU (160x144) en el
                
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop)
    {
        // TODO: bucle de frame. Algo así (adaptando al timing de 59.7275 Hz):
        //
        //   let now = Instant::now();
        //   if now >= self.next_frame {
        //       let mut ticks_frame = 17556u16;
        //       while ticks_frame > 0 {
        //           let mut ticks = self.emulator.cpu.handle_interrupts() as u16;
        //           if ticks == 0 && !self.emulator.cpu.halted {
        //               ticks = self.emulator.cpu.step() as u16;
        //           }
        //           self.emulator.ppu.step(ticks, &mut self.emulator.cpu.raw_memory);
        //           self.emulator.timer.step(ticks as u8, &mut self.emulator.cpu.raw_memory);
        //           self.emulator.read_joypad();
        //           ticks_frame -= ticks;
        //       }
        //       self.next_frame = now + self.frame_duration;
        //       // pedir redraw de la ventana
        //   }
    }
}

fn main()
{
    let path: &str = "path";
    let romdata = fs::read(path)
        .expect("No se pudo abrir el archivo");

    // El tipo de MBC y el numero de bancos los deduce la fabrica del cartucho
    let mut emulator = Emulator::new(romdata);
    emulator.cpu.raw_memory.address_bus[0xFF00] |= 0b11001111;

    let event_loop = EventLoop::new().unwrap();
    let context = Context::new(event_loop.owned_display_handle()).unwrap();

    let mut app = App::new(context, emulator);
    event_loop.run_app(&mut app).unwrap();
}
