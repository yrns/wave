use rayon::prelude::*;
use std::env;
use termplot::*;

fn main() -> Result<(), hound::Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <input.wav>", args[0]);
        std::process::exit(1);
    }
    let filename = &args[1];
    let mut reader = hound::WavReader::open(filename)?;

    //dbg!(reader.spec());

    // Convert samples to f64.
    let samples: Vec<f64> = match reader.spec().sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap() as f64).collect(),
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| (s.unwrap() as f64) / i16::MAX as f64)
            .collect(),
    };

    let id = || (f64::INFINITY, f64::NEG_INFINITY);

    let len = 1000;

    let (min, max) = samples[..len]
        .par_iter()
        .fold(id, |(min, max), a| (min.min(*a), max.max(*a)))
        .reduce(id, |a, b| (a.0.min(b.0), a.1.max(b.1)));

    println!("{} of {} samples:", len, samples.len());

    let mut plot = Plot::default();

    let plot = plot
        .set_domain(Domain(0f64..(len as f64)))
        .set_codomain(Domain(min as f64..max as f64))
        .set_size(Size::new(72, 48))
        .add_plot(Box::new(plot::Bars::new(samples)));

    print!("{plot}");

    Ok(())
}
