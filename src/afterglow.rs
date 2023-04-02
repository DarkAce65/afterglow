#![deny(clippy::all)]

mod led;

use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use led::LEDStrip;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{
    CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution,
};
use nokhwa::Camera;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{
    cmp::Ordering,
    f64::consts::{PI, TAU},
    thread,
    time::Duration,
};

fn prompt_camera_device() -> CameraIndex {
    let mut devices =
        nokhwa::query(nokhwa::utils::ApiBackend::Auto).expect("Unable to query video devices");
    if devices.is_empty() {
        panic!("No devices found");
    }

    devices.sort_by_key(|device| device.index().clone());
    let device_options: Vec<String> = devices
        .iter()
        .map(|device| format!("{} ({})", device.human_name(), device.description()))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a video input to capture from")
        .items(&device_options)
        .default(0)
        .interact()
        .expect("Must choose a video device to capture from");

    devices[selection].index().clone()
}

fn prompt_camera(camera_index: CameraIndex) -> Camera {
    let mut camera = Camera::new(
        camera_index,
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::None),
    )
    .expect("Unable to build camera");
    let camera_resolutions = camera
        .compatible_list_by_resolution(FrameFormat::YUYV)
        .expect("Unable to get available camera resolutions");

    let mut resolutions: Vec<&Resolution> = camera_resolutions.keys().collect();
    resolutions.sort_by(|a, b| match a.width().cmp(&b.width()) {
        Ordering::Equal => a.height().cmp(&b.height()),
        ord => ord,
    });
    let resolution_options: Vec<String> = resolutions
        .iter()
        .map(|resolution| {
            format!(
                "{}\t(fps options: {:?})",
                resolution,
                camera_resolutions.get(resolution).unwrap()
            )
        })
        .collect();
    let selected_resolution_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select capture resolution")
        .items(&resolution_options)
        .default(0)
        .interact()
        .expect("Must choose a resolution");

    let fps_options = camera_resolutions
        .get(resolutions[selected_resolution_index])
        .unwrap();
    let selected_fps_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select capture fps")
        .items(fps_options)
        .default(0)
        .interact()
        .expect("Must choose an fps option");

    camera
        .set_camera_requset(RequestedFormat::new::<RgbFormat>(
            RequestedFormatType::Closest(CameraFormat::new(
                *resolutions[selected_resolution_index],
                FrameFormat::YUYV,
                fps_options[selected_fps_index],
            )),
        ))
        .expect("Failed to set camera format");

    camera
}

fn build_segment_map(num_leds: usize, width: u32, height: u32) -> Vec<Option<usize>> {
    let mut segment_table: Vec<Option<usize>> =
        Vec::with_capacity((width * height).try_into().unwrap());

    let width = width as i32;
    let height = height as i32;
    let half_width = width / 2;
    let half_height = height / 2;
    let edge = half_width.min(half_height) / 2;

    let theta_scalar = (num_leds as f64) / TAU;

    for y in 0..height {
        let dy = (y - half_height) as f64;
        for x in 0..width {
            let dx = (half_width - x) as f64;
            segment_table.push(if dx.hypot(dy) >= edge.into() {
                let theta = dy.atan2(dx) + PI;
                let segment = ((theta * theta_scalar).floor() as usize).min(num_leds - 1);
                Some(segment)
            } else {
                None
            });
        }
    }

    segment_table
}

fn main() {
    let camera_index = prompt_camera_device();
    let mut camera = prompt_camera(camera_index);

    let resolution = camera.resolution();
    let width = resolution.width();
    let height = resolution.height();

    let segment_map = build_segment_map(NUM_LEDS, width, height);

    camera.open_stream().expect("Unable to open stream");

    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 16_000_000, Mode::Mode0)
        .expect("Unable to initialize SPI");

    const NUM_LEDS: usize = 36;
    let mut led_strip: LEDStrip<NUM_LEDS> = LEDStrip::new();

    let frame_delay = Duration::from_millis((1000 / camera.frame_rate()).into());

    loop {
        let frame = camera.frame().expect("Unable to get frame from camera");
        let decoded_image = frame.decode_image::<RgbFormat>().unwrap();

        let mut led_values: [(u64, u64, u64); NUM_LEDS] = [(0, 0, 0); NUM_LEDS];
        let mut counts: [u64; NUM_LEDS] = [0; NUM_LEDS];
        for (index, pixel) in decoded_image.chunks_exact(3).enumerate() {
            if let Some(segment) = segment_map[index] {
                if counts[segment] == 0 {
                    led_values[segment].0 = u64::from(pixel[0]).pow(2);
                    led_values[segment].1 = u64::from(pixel[1]).pow(2);
                    led_values[segment].2 = u64::from(pixel[2]).pow(2);
                } else {
                    led_values[segment].0 += u64::from(pixel[0]).pow(2);
                    led_values[segment].1 += u64::from(pixel[1]).pow(2);
                    led_values[segment].2 += u64::from(pixel[2]).pow(2);
                }
                counts[segment] += 1;
            }
        }

        for (index, led_value) in led_values.iter().enumerate() {
            let (r, g, b) = led_value;
            let count = counts[index];
            let r = ((r / count) as f64).sqrt() as u32;
            let g = ((g / count) as f64).sqrt() as u32;
            let b = ((b / count) as f64).sqrt() as u32;
            let color = r << 16 | g << 8 | b;
            led_strip.set_led(index, color);
        }

        spi.write(led_strip.get_spi_data())
            .expect("Failed to write SPI data");
        thread::sleep(frame_delay);
    }
}
