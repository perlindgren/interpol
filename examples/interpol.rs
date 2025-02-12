use core::f32::consts::TAU;
use realfft::num_complex::ComplexFloat;
use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use std::time::Instant;

fn main() {
    const FS: usize = 48000;
    const LENGTH: usize = 1000;
    const OVER_SAMPLING: usize = 10;
    const F1: f32 = 81.9;

    let length = FS * OVER_SAMPLING;

    println!("length {}", length);

    // make a planner
    let mut real_planner = RealFftPlanner::<f32>::new();

    // create a FFT
    let r2c = real_planner.plan_fft_forward(length);
    // make a dummy real-valued signal (filled with zeros)
    let mut indata = r2c.make_input_vec();

    // populate indata
    for (t, v) in indata[0..LENGTH].iter_mut().enumerate() {
        *v = (F1 * t as f32 * TAU / FS as f32).sin();
    }

    // hann filter
    hann_window_in_place(&mut indata[0..LENGTH]);

    // make a vector for storing the spectrum
    let mut spectrum = r2c.make_output_vec();

    let now = Instant::now();
    // forward transform the signal
    r2c.process(&mut indata, &mut spectrum).unwrap();

    println!("time = {:?}", now.elapsed());

    let max = spectrum
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
        .map(|(index, _)| index)
        .unwrap();

    println!("max {:?}", max);

    let cents = 1200.0 * (max as f32 / 824.0).log2();

    println!("cents {}", cents);
}

#[inline(always)]
pub fn hann_window_in_place(in_samples: &mut [f32]) {
    let n = in_samples.len();

    in_samples
        .iter_mut()
        .enumerate()
        .for_each(|(i, in_s)| *in_s = *in_s * 0.5 * (1.0 - (TAU * i as f32 / n as f32).cos()));
}
