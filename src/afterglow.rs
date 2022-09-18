#![deny(clippy::all)]

mod led;

use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use led::LEDStrip;
use nokhwa::{Camera, CameraFormat, FrameFormat, Resolution};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{cmp::Ordering, thread::sleep, time::Duration};

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

    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 16_000_000, Mode::Mode0)
        .expect("Unable to initialize SPI");

    let led_strip = LEDStrip { data: [0x0; 32] };
    loop {
        let data: Vec<u8> = led_strip
            .make_data_frames()
            .into_iter()
            .flat_map(|frame| <[u8; 4]>::from(frame))
            .collect();
        if !data.is_empty() {
            spi.write(&data).expect("Failed to write SPI data");
            sleep(Duration::from_millis(500));
        }
    }
}
