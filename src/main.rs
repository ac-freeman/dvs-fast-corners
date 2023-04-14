use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use clap::Parser;
use aedat::base::Decoder;
use aedat::events_generated::Event;
use ndarray::{Array, Array2};

/// Command line argument parser
#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]
pub struct MyArgs {
    /// Input .aedat4 file path
    #[clap(short, long)]
    pub(crate) input: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: MyArgs = MyArgs::parse();
    let file_path = args.input.as_str();

    // let bufreader = BufReader::new(File::open(file_path)?);
    let mut aedat_decoder = Decoder::new_from_file(Path::new(args.input.as_str()))?;

    let mut detector = FastDetector::new(true, 346, 260);

    loop {
        if let Some(packet_res) = aedat_decoder.next() {
            let packet = packet_res?;

            println!("{:?}", packet);
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
                println!("{:?}", event);
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
        // todo!()

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
