#![deny(clippy::all)]

use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use minifb::{Key, Window, WindowOptions};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{
    CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution,
};
use nokhwa::Camera;
use std::cmp::Ordering;
use std::f64::consts::{PI, TAU};
use std::{thread, time::Duration};

fn from_u64_rgb(r: u64, g: u64, b: u64) -> u32 {
    let (r, g, b): (u32, u32, u32) = (
        r.try_into().unwrap(),
        g.try_into().unwrap(),
        b.try_into().unwrap(),
    );
    (r << 16) | (g << 8) | b
}

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

fn start_visual_debugger(mut camera: Camera) {
    let resolution = camera.resolution();
    let width = resolution.width();
    let height = resolution.height();

    const NUM_LEDS: usize = 50;
    let segment_map = build_segment_map(NUM_LEDS, width, height);

    let width = width.try_into().unwrap();
    let height: usize = height.try_into().unwrap();
    let window_height = height * 2;

    let mut window: Window = Window::new(
        "afterglow",
        width,
        window_height,
        WindowOptions {
            title: false,
            borderless: true,
            ..WindowOptions::default()
        },
    )
    .unwrap();

    let frame_delay = Duration::from_millis((1000 / camera.frame_rate()).into());

    let mut source_image = Vec::with_capacity(width * height);
    for _ in 0..width * height {
        source_image.push(0);
    }

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame = camera.frame().expect("Unable to get frame from camera");
        let decoded_image = frame.decode_image::<RgbFormat>().unwrap();

        let mut led_values: [(u64, u64, u64); NUM_LEDS] = [(0, 0, 0); NUM_LEDS];
        let mut counts: [u64; NUM_LEDS] = [0; NUM_LEDS];
        for (index, pixel) in decoded_image.chunks_exact(3).enumerate() {
            let (r, g, b) = (
                u64::from(pixel[0]),
                u64::from(pixel[1]),
                u64::from(pixel[2]),
            );

            source_image[index] = from_u64_rgb(r, g, b);

            if let Some(segment) = segment_map[index] {
                if counts[segment] == 0 {
                    led_values[segment].0 = r.pow(2);
                    led_values[segment].1 = g.pow(2);
                    led_values[segment].2 = b.pow(2);
                } else {
                    led_values[segment].0 += r.pow(2);
                    led_values[segment].1 += g.pow(2);
                    led_values[segment].2 += b.pow(2);
                }
                counts[segment] += 1;
            }
        }

        let image_buffer: Vec<u32> = (0..(width * window_height))
            .map(|index| {
                if index < width * height {
                    match segment_map[index] {
                        Some(segment) => {
                            let (r, g, b) = led_values[segment];
                            let count = counts[segment];
                            from_u64_rgb(
                                ((r / count) as f64).sqrt() as u64,
                                ((g / count) as f64).sqrt() as u64,
                                ((b / count) as f64).sqrt() as u64,
                            )
                        }
                        None => 0,
                    }
                } else {
                    source_image[index - width * height]
                }
            })
            .collect();

        window
            .update_with_buffer(&image_buffer, width, window_height)
            .unwrap();

        thread::sleep(frame_delay);
    }
}

fn main() {
    let camera_index = prompt_camera_device();
    let mut camera = prompt_camera(camera_index);

    camera.open_stream().expect("Unable to open stream");

    start_visual_debugger(camera);
}
