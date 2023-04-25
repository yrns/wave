use std::path::Path;

use anyhow::Result;
//use rayon::prelude::*;
use image::*;
use notify_debouncer_mini::notify::*;
use softbuffer::GraphicsContext;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <input.wav>", args[0]);
        std::process::exit(1);
    }
    display(Path::new(&args[1])).unwrap();
}

fn load(path: &Path) -> Result<Vec<f64>, hound::Error> {
    let mut reader = hound::WavReader::open(path)?;

    println!("{:?}", reader.spec());

    // Convert samples to f64.
    let samples: Vec<_> = match reader.spec().sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap() as f64).collect(),
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| (s.unwrap() as f64) / (i16::MAX as f64))
            .collect(),
    };

    //let id = || (f64::INFINITY, f64::NEG_INFINITY);
    // let (min, max) = samples
    //     .par_iter()
    //     .fold(id, |(min, max), a| (min.min(*a), max.max(*a)))
    //     .reduce(id, |a, b| (a.0.min(b.0), a.1.max(b.1)));
    // println!("{} samples, min: {} max: {}", samples.len(), min, max);

    Ok(samples)
}

// Blank white.
pub fn white(width: u32, height: u32) -> RgbImage {
    RgbImage::from_vec(width, height, vec![0xff; (width * height * 3) as usize]).unwrap()
}

const BANDH: u32 = 64; // band height
const SPACE: u32 = 16; // space between bands

fn generate(samples: &[f64], width: u32, height: u32) -> RgbImage {
    let mut image = white(width, height);

    let mut y0 = 0;

    for (i, s) in samples.iter().enumerate() {
        let i = i as u32;
        let x = i % width;
        let y = i / width;
        let y = SPACE + y * (BANDH + SPACE) + BANDH / 2;

        // draw gray line as midpoint of the band
        match image.get_pixel_mut_checked(x, y) {
            Some(p) => p.0 = [0xee; 3],
            None => break,
        }

        // draw reddish if clipping?
        let color = if s.abs() > 1.0f64 {
            [0xff, 0x33, 0x33]
        } else {
            [0x33; 3]
        };
        let s = (s.clamp(-1.0, 1.0) * (BANDH / 2) as f64) as i32;
        let y = (y as i32 - s) as u32;

        // draw sample as a vertical line from the previous sample y
        let line = if y0 <= y { y0..=y } else { y..=y0 };
        for i in line {
            match image.get_pixel_mut_checked(x, i) {
                Some(p) => p.0 = color,
                None => continue,
            }
        }

        y0 = y;
    }

    image
}

fn display(path: &Path) -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;
    let mut graphics_context =
        unsafe { GraphicsContext::new(&window, &window) }.expect("graphics context");

    // Need to own it.
    let path = path.to_path_buf();
    let mut samples = load(&path).unwrap();

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(std::time::Duration::from_secs_f32(0.25), None, tx)
            .unwrap();
    debouncer
        .watcher()
        .watch(&path, RecursiveMode::NonRecursive)
        .unwrap();

    let proxy = event_loop.create_proxy();

    // Bounce watcher events to the event loop.
    std::thread::spawn(move || {
        while let Ok(e) = rx.recv() {
            match e {
                Ok(_) => {
                    if let Err(_) = proxy.send_event(()) {
                        // event loop closed
                        return;
                    }
                }
                Err(e) => eprintln!("watch error: {e:?}"),
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_wait();

        let redraw = match event {
            Event::UserEvent(_) => {
                samples = load(&path).unwrap();
                true
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => true,
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
                false
            }
            _ => false,
        };

        if redraw {
            let (width, height) = window.inner_size().into();

            // Resize to window width, keep height.
            let image = generate1(&samples, width);

            let mut bg = white(width, height);
            imageops::overlay(&mut bg, &image, 0, 0);

            // Convert to ARGB as u32:
            let buffer: Vec<_> = DynamicImage::ImageRgb8(bg)
                .to_rgba8()
                .pixels()
                .map(|p| {
                    let p: u32 = unsafe { std::mem::transmute(p.0) };
                    p
                })
                .collect();

            graphics_context.set_buffer(&buffer, width as u16, height as u16);
        }
    });
}

pub fn generate1(samples: &[f64], width: u32) -> RgbImage {
    let image = generate(samples, samples.len() as u32, BANDH + SPACE * 2);
    let height = image.height();
    DynamicImage::ImageRgb8(image)
        .resize_exact(width, height, imageops::FilterType::Triangle)
        .into_rgb8()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_png() {
        let samples = load(Path::new("sine.wav")).unwrap();
        let image = generate1(&samples, 640);
        image.save("sine.png").unwrap();
    }
}
