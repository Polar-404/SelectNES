use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapRb, traits::{Consumer, Split}};

pub struct AudioOutput {
    pub producer: ringbuf::HeapProd<f32>,
    _stream: cpal::Stream
}

impl AudioOutput {
    pub fn new() -> Option<(Self, u32)> {
        let host = cpal::default_host();
        let device = if let Some(d) = host.default_output_device() {
            d
        } else {
            eprintln!("[WARNING]: No audio output device found. The emulator will play without audio.");
            return None;
        };

        let config = if let Ok(c) = device.default_output_config() {
            c
        } else {
            eprintln!("[WARNING]: Failed to get default audio output config. The emulator will play without audio.");
            return None
        };

        let _sample_rate= config.sample_rate().0;
        let rb = HeapRb::<f32>::new(4016);
        let (producer, mut consumer) = rb.split();
        let mut last_sample: f32 = 0.0;

        let stream = if let Ok(s) = device.build_output_stream(
            &config.into(), 
            move |data: &mut [f32], _| {
                for frame in data.chunks_mut(2) {
                    let sample = consumer.try_pop().unwrap_or(last_sample);
                    last_sample = sample;
                    frame[0] = sample;
                    frame[1] = sample;
                }
            }, |err| {
                eprintln!("[WARNING]: Failed to create an audio stream. The emulator will play without audio. {:?}", err)
            }, None) {   
            s
        } else {
            eprintln!("[WARNING]: Build output stream failed. No audio devide");
            return None
        };
        if let Err(e) = stream.play() {
            eprintln!("[WARNING]: Error while trying to play audio stream. The emulator will play without audio. {:?}", e);
            return None
        }

        Some(
            (
                AudioOutput {
                    producer,
                    _stream: stream
                }, 
                _sample_rate
            )
        )
    }
}