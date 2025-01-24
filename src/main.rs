use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};

use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

struct GameOfLife {
    cells_current: Vec<bool>,
    cells_next: Option<Vec<bool>>,
    width: u32,
    height: u32,
}

impl GameOfLife {
    fn new(width: u32, height: u32) -> Self {
        Self {
            cells_current: vec![false; (width * height) as usize],
            cells_next: Some(vec![false; (width * height) as usize]),
            width,
            height,
        }
    }

    fn set_cell(&mut self, x: u32, y: u32, value: bool) {
        self.cells_current[(x + y * self.width) as usize] = value;
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (x + y * self.width) as usize
    }

    fn count_alive_neighbors(&self, x: u32, y: u32) -> u8 {
        let mut count = 0;

        // Iterate through the 3x3 grid around the cell
        for dy in -1..=1 {
            for dx in -1..=1 {
                // Skip the center cell
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = (x as i32 + dx) as u32;
                let ny = (y as i32 + dy) as u32;

                // Check if the neighbor is alive and within bounds
                if nx < self.width && ny < self.height && self.cells_current[self.index(nx, ny)] {
                    count += 1;
                }
            }
        }

        count
    }
}

impl App for GameOfLife {
    fn tick(&mut self) {
        // let start = Instant::now();

        let width = self.width;

        let mut cells_next = self.cells_next.take().unwrap();

        cells_next
            .par_iter_mut()
            .enumerate()
            .for_each(|(index, cell)| {
                let x = index as u32 % width;
                let y = index as u32 / width;

                let alive_neighbors = self.count_alive_neighbors(x, y);

                // Apply the rules of the Game of Life
                *cell = match (self.cells_current[index], alive_neighbors) {
                    (true, 2) | (true, 3) => true, // Stay alive
                    (false, 3) => true,            // Become alive
                    _ => false,                    // Otherwise, die
                };
            });

        self.cells_next = Some(cells_next);
        std::mem::swap(&mut self.cells_current, self.cells_next.as_mut().unwrap());

        // println!("{:?}", start.elapsed());
    }

    fn draw(&self, pixels: &mut [u32]) {
        pixels
            .par_iter_mut()
            .enumerate()
            .for_each(|(index, pixel)| {
                let x = (index * 3) as u32 % self.width;
                let y = (index * 3) as u32 / self.width;

                let mut color = 0xFF000000;

                if self.cells_current[self.index(x, y)] {
                    color += 0xFF0000;
                }

                if self.cells_current[self.index(x + 1, y)] {
                    color += 0xFF00;
                }

                if self.cells_current[self.index(x + 2, y)] {
                    color += 0xFF;
                }

                *pixel = color;
            });
    }
}

fn main() {
    let mut game = GameOfLife::new(2048 * 3, 1024);

    game.set_cell(100, 100, true);
    game.set_cell(101, 101, true);
    game.set_cell(102, 101, true);
    game.set_cell(102, 100, true);
    game.set_cell(102, 99, true);

    run(game, "Subpixel Game of Life", 2048, 1024);
}

trait App {
    fn tick(&mut self);
    fn draw(&self, pixels: &mut [u32]);
}

fn run(mut app: impl App, title: impl ToString, width: u32, height: u32) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let window = WindowBuilder::new()
        .with_title(title.to_string())
        .with_inner_size(PhysicalSize::new(width, height))
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let context = softbuffer::Context::new(&window).unwrap();
    let mut surface = softbuffer::Surface::new(&context, &window).unwrap();

    let mut next_frame = Instant::now();
    let frame_time = Duration::from_secs_f32(1.0 / 144.0);

    let mut paused = false;

    event_loop
        .run(|event, target| match event {
            Event::AboutToWait => {
                if Instant::now() >= next_frame && !paused {
                    app.tick();
                    window.request_redraw();
                    next_frame += frame_time;
                }

                target.set_control_flow(ControlFlow::WaitUntil(next_frame));
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(_) => {
                    window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();

                    surface
                        .resize(
                            NonZeroU32::new(size.width).unwrap(),
                            NonZeroU32::new(size.height).unwrap(),
                        )
                        .unwrap();

                    let mut surface = surface.buffer_mut().unwrap();

                    app.draw(&mut surface);

                    window.pre_present_notify();
                    surface.present().unwrap();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let keycode = match event.physical_key {
                        PhysicalKey::Code(keycode) => keycode,
                        PhysicalKey::Unidentified(_) => panic!(),
                    };

                    if keycode == KeyCode::Space && !event.state.is_pressed() {
                        paused = !paused;
                    }
                }
                WindowEvent::CloseRequested => target.exit(),
                _ => {}
            },
            _ => {}
        })
        .unwrap();
}
