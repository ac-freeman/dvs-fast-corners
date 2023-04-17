# dvs-fast-corners

This is a Rust implementation and visualization of the [Fast Corners](https://github.com/uzh-rpg/rpg_corner_events) algorithm for the Dynamic Vision Sensor (DVS) camera.

Based on the work of:

E. Mueggler, C. Bartolozzi, D. Scaramuzza:
**Fast Event-based Corner Detection.**
British Machine Vision Conference (BMVC), London, 2017.

Find the original paper [here](http://rpg.ifi.uzh.ch/docs/BMVC17_Mueggler.pdf). 

## Setup and Usage

- [Install Rust](https://www.rust-lang.org/tools/install)
- Clone this repository and `cd` into it
- Run `cargo run --release -- --input "/path/to/aedat4/file"`

Setup is simple, with no non-Rust requirements.

Out of the box, this works with `.aedat4` files created with iniVation cameras. The default resolution is `346x260`, but you can easily change this to suit your needs.

