#![cfg(feature = "feature-logging")]
/// Logging tools for comparing features. Copied from https://github.com/ac-freeman/adder-codec-rs/blob/feature-eval-log/adder-codec-rs/src/utils/logging.rs
use crate::{FastDetector, MyArgs, HEIGHT, WIDTH};
use aedat::base::Decoder;
use chrono::Local;
use image::{ImageBuffer, Rgb};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use show_image::{create_window, WindowOptions};
use std::collections::HashSet;
use std::error::Error;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct LogFeature {
    pub x: u16,
    pub y: u16,
    pub non_max_suppression: bool,
    pub source: LogFeatureSource,
}

impl Serialize for LogFeature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("LogFeature", 4)?;
        state.serialize_field("x", &self.x)?;
        state.serialize_field("y", &self.y)?;
        state.serialize_field("n", &self.non_max_suppression)?;
        state.serialize_field("s", &self.source)?;
        state.end()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize)]
pub enum LogFeatureSource {
    ADDER,
    OpenCV,
    DVS,
}

impl LogFeature {
    pub fn from_coord(x: i16, y: i16) -> Self {
        Self {
            x: x as u16,
            y: y as u16,
            non_max_suppression: false,
            source: LogFeatureSource::DVS,
        }
    }
}

pub fn logging_main(args: MyArgs) -> Result<(), Box<dyn Error>> {
    let mut aedat_decoder = Decoder::new_from_file(Path::new(args.input.as_str()))?;

    let mut detector = FastDetector::new(HEIGHT, WIDTH);

    // Create an Image for showing the live event view
    let mut img_events: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::new(WIDTH as u32, HEIGHT as u32);
    let mut running_t = None;
    let frame_interval_t = 1e6 as i64 / 60; // 60 fps

    let window = create_window(
        "image-dvs",
        WindowOptions {
            preserve_aspect_ratio: true,
            size: Some([WIDTH as u32 * 2, HEIGHT as u32 * 2]),
            ..Default::default()
        },
    )?;

    let date_time = Local::now();
    let formatted = format!("features_{}.log", date_time.format("%d_%m_%Y_%H_%M_%S"));
    let mut log_handle = std::fs::File::create(formatted).ok();

    if let Some(handle) = &mut log_handle {
        writeln!(handle, "{}x{}x{}", WIDTH, HEIGHT, 1).unwrap();
    }

    let mut features = HashSet::new();

    loop {
        if let Some(packet_res) = aedat_decoder.next() {
            let packet = packet_res?;
            if packet.stream_id != 0 {
                continue;
            }

            let event_packet =
                match aedat::events_generated::size_prefixed_root_as_event_packet(&packet.buffer) {
                    Ok(result) => result,
                    Err(_) => {
                        panic!("the packet does not have a size prefix");
                    }
                };

            let event_arr = match event_packet.elements() {
                None => continue,
                Some(events) => events,
            };

            let start = Instant::now();

            for event in event_arr {
                if detector.is_feature(event, 1) {
                    features.insert((event.x(), event.y()));
                } else {
                    features.remove(&(event.x(), event.y()));
                }
            }

            let total_duration_nanos = start.elapsed().as_nanos();
            if let Some(handle) = &mut log_handle {
                for (x, y) in &features {
                    let bytes =
                        serde_pickle::to_vec(&LogFeature::from_coord(*x, *y), Default::default())
                            .unwrap();
                    handle.write_all(&bytes).unwrap();
                }

                let out = format!("\nDVS FAST: {}", total_duration_nanos);
                handle
                    .write_all(&serde_pickle::to_vec(&out, Default::default()).unwrap())
                    .unwrap();
            }

            for event in event_arr {
                match running_t {
                    None => running_t = Some(event.t()),
                    Some(t) if event.t() > t + frame_interval_t => {
                        running_t = Some(event.t());

                        for (x, y) in &features {
                            // Color the pixels in a + centered on it white
                            let radius = 2;
                            for i in -radius..=radius {
                                img_events
                                    .get_pixel_mut((*x as i32 + i) as u32, (*y as i32) as u32)
                                    .0 = [255, 255, 255];
                                img_events
                                    .get_pixel_mut((*x as i32) as u32, (*y as i32 + i) as u32)
                                    .0 = [255, 255, 255];
                            }
                        }

                        window.set_image("image-dvs", img_events.clone())?;
                        img_events = ImageBuffer::new(WIDTH as u32, HEIGHT as u32);
                    }
                    Some(_) => {
                        let color_idx = if event.on() { 0 } else { 1 };
                        img_events
                            .get_pixel_mut(event.x() as u32, event.y() as u32)
                            .0[color_idx] = 255;
                    }
                }
            }
        } else {
            break;
        }
    }
    Ok(())
}
