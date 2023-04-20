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

                let mut buffer: Vec<u32> = vec![0x00ffffff; (width * height) as usize];

                // band height
                let bandh = 128u32;

                for (i, s) in samples.iter().enumerate() {
                    let i = i as u32;
                    let x = i % width;
                    let y = i / width;
                    let y = y * bandh + bandh / 2;

                    let index = (x + y * width) as usize;
                    if index >= buffer.len() {
                        break;
                    } else {
                        // draw gray line as midpoint of the band
                        buffer[index] = 0x00cccccc;
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

    #[test]
    fn it_works() {
        display(load("sine.wav").unwrap()).unwrap();
    }
}
