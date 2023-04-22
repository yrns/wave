use anyhow::Result;
//use rayon::prelude::*;
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
    let samples = load(&args[1]).unwrap();
    display(samples).unwrap();
}

fn load(path: &str) -> Result<Vec<f64>, hound::Error> {
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

fn generate(samples: &[f64], width: u32, height: u32) -> Vec<u32> {
    let mut buffer: Vec<u32> = vec![0xffffffff; (width * height) as usize];

    let bandh = 64u32; // band height
    let space = 16; // space between bands

    for (i, s) in samples.iter().enumerate() {
        let i = i as u32;
        let x = i % width;
        let y = i / width;
        let y = space + y * (bandh + space) + bandh / 2;

        let index = (x + y * width) as usize;
        if index >= buffer.len() {
            break;
        } else {
            // draw gray line as midpoint of the band
            buffer[index] = 0x00eeeeee;
        }

        // draw reddish if clipping?
        let color = if s.abs() > 1.0f64 {
            0x00ff3333
        } else {
            0x00333333
        };
        let s = (s.clamp(-1.0, 1.0) * (bandh / 2) as f64) as i32;
        let y = (y as i32 - s) as u32;

        let index = (x + y * width) as usize;
        if index >= buffer.len() {
            continue;
        } else {
            // draw sample
            buffer[index] = color;
        }
    }

    buffer
}

fn display(samples: Vec<f64>) -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop)?;
    let mut graphics_context =
        unsafe { GraphicsContext::new(&window, &window) }.expect("graphics context");

    //let mut size0 = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let size = window.inner_size();

                // dbg!("redraw", size);
                // if Some(size) != size0 {
                //     size0 = Some(size);
                //     return;
                // }

                let (width, height) = (size.width, size.height);

                let buffer = generate(&samples, width, height);

                graphics_context.set_buffer(&buffer, width as u16, height as u16);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    // use std::fs::File;
    // use std::io::BufWriter;
    // use std::path::Path;

    #[test]
    fn generate_png() {
        let samples = load("sine.wav").unwrap();
        let buffer = generate(&samples, 1024, 720);

        // Convert to bytes.
        let mut data: Vec<u8> = Vec::new();
        for color in buffer {
            data.extend(&color.to_ne_bytes()[0..=2])

            // match color.to_ne_bytes() {
            //     c @ [r, g, b, a] => {
            //         if r < 255 || g < 255 || b < 255 {
            //             println!("{a} {r} {g} {b}")
            //         }
            //         data.push(r);
            //         data.push(g);
            //         data.push(b);
            //     }
            // }
        }

        // let path = Path::new("sine.png");
        // let file = File::create(path).unwrap();
        // let mut w = BufWriter::new(file);

        // let mut encoder = png::Encoder::new(&mut w, 1024, 720);
        // encoder.set_color(png::ColorType::Rgb);
        // encoder.set_depth(png::BitDepth::Eight);
        // let mut writer = encoder.write_header().unwrap();
        // writer.write_image_data(&data).unwrap();

        image::save_buffer("sine.png", &data, 1024, 720, image::ColorType::Rgb8).unwrap();
    }
}
