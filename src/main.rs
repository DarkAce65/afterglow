#![deny(clippy::all)]

use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use minifb::{Key, Window, WindowOptions};
use nokhwa::{Camera, CameraFormat, FrameFormat, Resolution};
use rayon::prelude::*;
use std::{cmp::Ordering, thread, time::Duration};

fn from_u8_rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    (r << 16) | (g << 8) | b
}

fn prompt_camera_device() -> usize {
    let mut devices = nokhwa::query().expect("Unable to query video devices");
    if devices.is_empty() {
        panic!("No devices found");
    }

    devices.sort_by_key(|device| device.index());
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

    devices[selection].index()
}

fn prompt_camera(camera_index: usize) -> Camera {
    let mut camera = Camera::new(camera_index, None).expect("Unable to build camera");
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
        .expect("Must choose an fps opiton");

    camera
        .set_camera_format(CameraFormat::new(
            *resolutions[selected_resolution_index],
            FrameFormat::YUYV,
            fps_options[selected_fps_index],
        ))
        .expect("Failed to set camera format");

    camera
}

fn main() {
    let camera_index = prompt_camera_device();
    let mut camera = prompt_camera(camera_index);

    camera.open_stream().expect("Unable to open stream");

    let resolution = camera.resolution();
    let width: usize = resolution.width().try_into().unwrap();
    let height: usize = resolution.height().try_into().unwrap();

    let mut window: Window = Window::new(
        "afterglow",
        width,
        height,
        WindowOptions {
            title: false,
            borderless: true,
            ..WindowOptions::default()
        },
    )
    .unwrap();

    let frame_delay = Duration::from_millis((1000 / camera.frame_rate()).into());
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame = camera.frame().expect("Unable to get frame from camera");
        let image_buffer: Vec<u32> = frame
            .as_raw()
            .par_chunks_exact(3)
            .map(|pixel| from_u8_rgb(pixel[0], pixel[1], pixel[2]))
            .collect();
        window
            .update_with_buffer(&image_buffer, width, height)
            .unwrap();

        thread::sleep(frame_delay);
    }
}
