# dvs-fast-corners

This is a Rust implementation and visualization of the [Fast Corners](https://github.com/uzh-rpg/rpg_corner_events) algorithm for the Dynamic Vision Sensor (DVS) camera.

Based on the work of:

E. Mueggler, C. Bartolozzi, D. Scaramuzza:
**Fast Event-based Corner Detection.**
British Machine Vision Conference (BMVC), London, 2017.

Find the original paper [here](http://rpg.ifi.uzh.ch/docs/BMVC17_Mueggler.pdf). 


![Screenshot from 2023-04-17 17-01-36-1](https://user-images.githubusercontent.com/19912588/232610281-9a616bae-06c5-4a28-8a31-793967b34230.png)
:-------------------------:
Features detected over 1/60th of a second are marked with a `+`. Note the predominance around the corners of the object.

## Setup and Usage

- [Install Rust](https://www.rust-lang.org/tools/install)
- Clone this repository and `cd` into it
- Run `cargo run --release -- --input "/path/to/aedat4/file"`
- Log the detected features to a file with `cargo run --features "feature-logging" --release -- --input "/path/to/aedat4/file"`

Setup is simple, with no non-Rust requirements.

Out of the box, this works with `.aedat4` files created with iniVation cameras. The default resolution is `346x260`, but you can easily change this to suit your needs.

