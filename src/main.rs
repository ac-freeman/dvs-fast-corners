use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use clap::Parser;
use aedat::base::Decoder;
use aedat::events_generated::Event;
use image::{ImageBuffer, Rgb};
use ndarray::{Array, Array2};
use show_image::create_window;

/// Command line argument parser
#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]
pub struct MyArgs {
    /// Input .aedat4 file path
    #[clap(short, long)]
    pub(crate) input: String,
}

#[show_image::main]
fn main() -> Result<(), Box<dyn Error>> {
    let args: MyArgs = MyArgs::parse();
    let file_path = args.input.as_str();

    // let bufreader = BufReader::new(File::open(file_path)?);
    let mut aedat_decoder = Decoder::new_from_file(Path::new(args.input.as_str()))?;

    let mut detector = FastDetector::new(true, 260, 346);

    // Create an Image from the detector's sae
    let mut img = ImageBuffer::new(346, 260);

    // Create an Image for showing the live event view
    let mut img_events: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(346, 260);
    let mut running_t = None;
    let mut frame_interval_t = 1e6 as i64 / 60;  // 60 fps

    let window = create_window("image", Default::default())?;
    let window_2 = create_window("image-dvs", Default::default())?;


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

            for event in event_arr {
                match running_t {
                    None => running_t = Some(event.t()),
                    Some(t) if event.t() > t + frame_interval_t  => {
                        running_t = Some(event.t());
                        // Display the image with show-image crate
                        window_2.set_image("image-dvs", img_events.clone())?;
                        img_events = ImageBuffer::new(346, 260);
                    }
                    Some(t) => {
                        let color_idx = if event.on() { 0 } else { 1 };
                        img_events.get_pixel_mut(event.x() as u32, event.y() as u32).0[color_idx] = 255;
                    }
                }

                if detector.is_feature(event, 1) {
                    // println!("feature");

                    // Convert the detector's sae to an Image for display
                    for (x, y, pixel) in img.enumerate_pixels_mut() {
                        *pixel = image::Rgb([detector.sae_[0][[y as usize, x as usize]] as u8, detector.sae_[0][[y as usize, x as usize]] as u8, detector.sae_[0][[y as usize, x as usize]] as u8]);
                    }

                    // Display the image with show-image crate
                    // window.set_image("image-001", img.clone())?;
                    // Update
                }

            }
        } else {
            break;
        }
    }

    println!("Hello, world!");

    Ok(())
}

pub struct FastDetector {
    detector_name_: String,
    sae_: [Array2<f64>; 2],
    circle3_: Vec<[i16; 2]>,
    circle4_: Vec<[i16; 2]>,
}

impl FastDetector {
    pub fn new(connect: bool, sensor_height: usize, sensor_width: usize) -> Self {
        let mut sae_ = [
            Array::zeros((sensor_height, sensor_width)),
            Array::zeros((sensor_height, sensor_width)),
        ];

        let circle3_ = vec![
            [0, 3], [1, 3], [2, 2], [3, 1],
            [3, 0], [3, -1], [2, -2], [1, -3],
            [0, -3], [-1, -3], [-2, -2], [-3, -1],
            [-3, 0], [-3, 1], [-2, 2], [-1, 3]
        ];

        let circle4_ = vec![
            [0, 4], [1, 4], [2, 3], [3, 2],
            [4, 1], [4, 0], [4, -1], [3, -2],
            [2, -3], [1, -4], [0, -4], [-1, -4],
            [-2, -3], [-3, -2], [-4, -1], [-4, 0],
            [-4, 1], [-3, 2], [-2, 3], [-1, 4]
        ];

        Self {
            detector_name_: "FAST".to_string(),
            sae_,
            circle3_,
            circle4_,
        }
    }

    fn is_border(&self, x: usize, y: usize, max_scale: usize) -> bool {
        let cs = max_scale * 4;
        x < cs || x >= self.sae_[0].shape()[1] as usize - cs ||
            y < cs || y >= self.sae_[0].shape()[0] as usize - cs
    }

    fn is_feature(&mut self, e: &Event, max_scale: usize) -> bool {

        // Update SAE.
        let pol = if e.on() { 1 } else { 0 };
        self.sae_[pol][(e.y() as usize, e.x() as usize)] = e.t() as f64;

        if self.is_border(e.x() as usize, e.y() as usize, max_scale) {
            return false;
        }

        let mut found_streak = false;

        for i in 0..16 {
            for streak_size in 3..=6 {

                // Check that streak event is larger than neighbor.
                if self.sae_[pol][[(e.y() + self.circle3_[i][1]) as usize, (e.x() + self.circle3_[i][0]) as usize]]
                    < self.sae_[pol][[(e.y() + self.circle3_[(i + 15) % 16][1]) as usize, (e.x() + self.circle3_[(i + 15) % 16][0]) as usize]] {
                    continue;
                }

                // Check that streak event is larger than neighbor.
                if self.sae_[pol][[(e.y() + self.circle3_[(i + streak_size - 1) % 16][1]) as usize, (e.x() + self.circle3_[(i+ streak_size - 1) % 16][0]) as usize]]
                    < self.sae_[pol][[(e.y() + self.circle3_[(i + streak_size) % 16][1]) as usize, (e.x() + self.circle3_[(i + streak_size) % 16][0]) as usize]] {
                    continue;
                }

                let mut min_t = self.sae_[pol][[(e.y() + self.circle3_[i][1]) as usize, (e.x() + self.circle3_[i][0]) as usize]];
                for j in 1..streak_size {
                    let tj = self.sae_[pol][[(e.y()+self.circle3_[(i+j)%16][1]) as usize, (e.x()+self.circle3_[(i+j)%16][0]) as usize]];
                    if tj < min_t {
                        min_t = tj;
                    }
                }

                let mut did_break = false;

                for j in streak_size..16{
                    let tj = self.sae_[pol][[(e.y()+self.circle3_[(i+j)%16][1]) as usize, (e.x()+self.circle3_[(i+j)%16][0]) as usize]];
                    if tj >= min_t {
                        did_break = true;
                        break;
                    }
                }

                if !did_break {
                    found_streak = true;
                    break;
                }

            }
            if found_streak {
                break;
            }
        }

        if found_streak {
            found_streak = false;
            for i in 0..20 {
                for streak_size in 4..=8 {
                    // Check that first event is larger than neighbor
                    if self.sae_[pol][[(e.y() + self.circle4_[i][1]) as usize, (e.x() + self.circle4_[i][0]) as usize]]
                        < self.sae_[pol][[(e.y() + self.circle4_[(i + 19) % 20][1]) as usize, (e.x() + self.circle4_[(i + 19) % 20][0]) as usize]] {
                            continue;
                        }

                    // Check that streak event is larger than neighbor
                    if self.sae_[pol][[(e.y() + self.circle4_[(i + streak_size - 1) % 20][1]) as usize, (e.x() + self.circle4_[(i + streak_size - 1) % 20][0]) as usize]]
                        < self.sae_[pol][[(e.y() + self.circle4_[(i + streak_size) % 20][1]) as usize, (e.x() + self.circle4_[(i + streak_size) % 20][0]) as usize]] {
                            continue;
                        }

                    let mut min_t = self.sae_[pol][[(e.y() + self.circle4_[i][1]) as usize, (e.x() + self.circle4_[i][0]) as usize]];
                    for j in 1..streak_size {
                        let tj = self.sae_[pol][[(e.y() + self.circle4_[(i + j) % 20][1]) as usize, (e.x() + self.circle4_[(i + j) % 20][0]) as usize]];
                        if tj < min_t {
                            min_t = tj;
                        }
                    }

                    let mut did_break = false;
                    for j in streak_size..20 {
                        let tj = self.sae_[pol][[(e.y() + self.circle4_[(i + j) % 20][1]) as usize, (e.x() + self.circle4_[(i + j) % 20][0]) as usize]];
                        if tj >= min_t {
                            did_break = true;
                            break;
                        }
                    }

                    if !did_break {
                        found_streak = true;
                        break;
                    }
                }
                if (found_streak) {
                    break;
                }
            }
        }

        found_streak
    }
}
