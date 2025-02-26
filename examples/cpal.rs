//! Feeds back the input stream directly into the output stream.
//!
//! Assumes that the input and output devices can use the same stream configuration and that they
//! support the f32 sample format.
//!
//! Uses a delay of `LATENCY_MS` milliseconds in case the default input and output streams are not
//! precisely synchronised.

use clap::Parser;
use core::f32::consts::TAU;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use interpol::hann_window_in_place;
use realfft::num_complex::ComplexFloat;
use realfft::RealFftPlanner;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(version, about = "CPAL feedback example", long_about = None)]
struct Opt {
    /// The input audio device to use
    #[arg(short, long, value_name = "IN", default_value_t = String::from("default"))]
    input_device: String,

    /// The output audio device to use
    #[arg(short, long, value_name = "OUT", default_value_t = String::from("default"))]
    output_device: String,

    /// Specify the delay between input and output
    #[arg(short, long, value_name = "DELAY_MS", default_value_t = 150.0)]
    latency: f32,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    let host;

    // cpal::default_host();
    // Support Windows and OSX later?

    #[cfg(target_os = "linux")]
    {
        host = cpal::host_from_id(cpal::HostId::Jack).expect("failed to initialize Jack host");
    }

    // Find devices.
    let input_device = if opt.input_device == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == opt.input_device).unwrap_or(false))
    }
    .expect("failed to find input device");

    let output_device = if opt.output_device == "default" {
        host.default_output_device()
    } else {
        host.output_devices()?
            .find(|x| x.name().map(|y| y == opt.output_device).unwrap_or(false))
    }
    .expect("failed to find output device");

    println!("Using input device: \"{}\"", input_device.name()?);
    println!("Using output device: \"{}\"", output_device.name()?);

    // We'll try and use the same configuration between streams to keep it simple.
    let mut config: cpal::StreamConfig = input_device.default_input_config()?.into();
    config.channels = 1;

    // Create a delay in case the input and output devices aren't synced.
    let latency_frames = (opt.latency / 1_000.0) * config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;

    // The buffer to share samples, between audio input and output
    let ring = HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();

    let ring2 = HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer2, mut consumer2) = ring2.split();

    // Fill the samples with 0.0 equal to the length of the delay.
    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.try_push(0.0).unwrap();
    }

    let fs = config.sample_rate.0;
    println!("sample rate {}", fs);

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        // println!("data.len {}", data.len());
        let mut output_fell_behind = false;
        for &s in data {
            if producer.try_push(s).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("output stream fell behind: try increasing latency");
        }

        let mut output2_fell_behind = false;
        for &s in data {
            if producer2.try_push(s).is_err() {
                output2_fell_behind = true;
            }
        }
        if output2_fell_behind {
            eprintln!("output stream fell behind: try increasing latency");
        }
    };

    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        // println!("out {}", data.len());
        let mut input_fell_behind = false;
        for sample in data {
            *sample = match consumer.try_pop() {
                Some(s) => s,
                None => {
                    input_fell_behind = true;
                    0.0
                }
            };
        }
        if input_fell_behind {
            eprintln!("input stream fell behind: try increasing latency");
        }
    };

    // Build streams.
    println!(
        "Attempting to build both streams with f32 samples and `{:?}`.",
        config
    );
    let input_stream = input_device.build_input_stream(&config, input_data_fn, err_fn, None)?;
    let output_stream = output_device.build_output_stream(&config, output_data_fn, err_fn, None)?;
    println!("Successfully built streams.");

    // Play the streams.
    println!(
        "Starting the input and output streams with `{}` milliseconds of latency.",
        opt.latency
    );
    input_stream.play()?;
    output_stream.play()?;

    const FS: usize = 48000;
    const CHUNK: usize = 500;
    const OVER_SAMPLING: usize = 20;

    let length = FS * OVER_SAMPLING;

    println!("length {}", length);

    // make a planner
    let mut real_planner = RealFftPlanner::<f32>::new();

    // create a FFT
    let r2c = real_planner.plan_fft_forward(length);
    // make a dummy real-valued signal (filled with zeros)
    let mut fft_in_data = [0.0f32; FS * OVER_SAMPLING];
    let mut in_buffer = [0.0f32; FS];
    let mut in_ptr = FS - CHUNK;

    // make a vector for storing the spectrum
    let mut spectrum = r2c.make_output_vec();
    let mut scratch = spectrum.clone();

    let note_hz = [
        82.41,  // Hz Low E (6th string)
        110.00, //  Hz A (5th string)
        146.83, //  Hz D (4th string)
        196.00, // Hz G (3rd string)
        246.94, // Hz B (2nd string)
        329.63, // Hz High E (1st string)
    ];

    loop {
        if let Some(s) = consumer2.try_pop() {
            // populate in_data

            in_buffer[in_ptr] = s;
            in_ptr += 1;
            if in_ptr == FS {
                in_ptr = FS - CHUNK;

                // hann filter
                hann_window_in_place(&mut fft_in_data[0..FS]);
                // forward transform the signal
                fft_in_data[0..FS].copy_from_slice(&in_buffer[..]);
                fft_in_data[FS..].iter_mut().for_each(|d| *d = 0.0);

                r2c.process_with_scratch(&mut fft_in_data, &mut spectrum, &mut scratch)
                    .unwrap();

                // find fundamental
                const THRESHOLD: f32 = 6.0;

                let mut best_abs = 0.0;
                let mut best_index = 0;
                for (index, bin_value) in spectrum[0..500 * OVER_SAMPLING].iter().enumerate() {
                    let abs = bin_value.abs();

                    // if (index - best_index < 5 && abs > best_abs) || abs > THRESHOLD * best_abs {
                    //     best_abs = abs;
                    //     best_index = index;
                    // }
                    if abs > best_abs {
                        best_abs = abs;
                        best_index = index;
                    }
                }

                let fundamental_freq = best_index as f32 / OVER_SAMPLING as f32;

                // find matching string
                let mut best_index = 0;
                let mut best_diff = 1000.0;
                for (index, freq) in note_hz.iter().enumerate() {
                    let diff = (fundamental_freq - *freq).abs();
                    if diff < best_diff {
                        best_index = index;
                        best_diff = diff;
                    }
                }

                let target_freq = note_hz[best_index];

                let cents = 1200.0 * (fundamental_freq / target_freq).log2();
                println!(
                    "string #{}, \tfreq {} \tfreq {:.2} \tcents {:.2}",
                    best_index, target_freq, fundamental_freq, cents
                );
                in_buffer.copy_within(CHUNK.., 0);
            }
        }
    }

    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}
