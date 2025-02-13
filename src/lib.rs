use std::f32::consts::TAU;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;

#[inline(always)]
pub fn hann_window_in_place(in_samples: &mut [f32]) {
    let n = in_samples.len();

    in_samples
        .iter_mut()
        .enumerate()
        .for_each(|(i, in_s)| *in_s = *in_s * 0.5 * (1.0 - (TAU * i as f32 / n as f32).cos()));
}

pub fn write_to_file(path: &str, data: &[f32]) {
    let mut f = File::create(path).expect("Unable to create file");
    for i in data {
        writeln!(f, "{}", i).expect("Unable to write data");
    }
}
