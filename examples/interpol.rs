use core::f32::consts::TAU;
use interpol::*;
use realfft::num_complex::ComplexFloat;
use realfft::RealFftPlanner;

use std::time::Instant;

fn main() {
    const FS: usize = 48000;
    const LENGTH: usize = 1000;
    const OVER_SAMPLING: usize = 10;
    const F1: f32 = 82.4;
    const F2: f32 = 164.8;

    let length = FS * OVER_SAMPLING;

    println!("length {}", length);

    // make a planner
    let mut real_planner = RealFftPlanner::<f32>::new();

    // create a FFT
    let r2c = real_planner.plan_fft_forward(length);
    // make a dummy real-valued signal (filled with zeros)
    let mut in_data = r2c.make_input_vec();

    // populate in_data
    for (t, v) in in_data[0..LENGTH].iter_mut().enumerate() {
        *v =
            (F1 * t as f32 * TAU / FS as f32).sin() + 0.5 * (F2 * t as f32 * TAU / FS as f32).sin();
    }

    // hann filter
    hann_window_in_place(&mut in_data[0..LENGTH]);

    // make a vector for storing the spectrum
    let mut spectrum = r2c.make_output_vec();

    let now = Instant::now();
    // forward transform the signal
    r2c.process(&mut in_data, &mut spectrum).unwrap();

    println!("time = {:?}", now.elapsed());

    let abs: Vec<_> = spectrum.iter().map(|c| c.abs()).collect();

    let max = abs
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(index, _)| index)
        .unwrap();

    println!("max {:?}", max);

    let cents = 1200.0 * (max as f32 / 824.0).log2();

    println!("cents {}", cents);
    write_to_file("octave/fft.txt", abs.as_slice());
}
