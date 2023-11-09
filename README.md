# ROS (Rust Operating System)
My very own OS written in Rust.

# Table of Contents
- [ROS (Rust Operating System)](#ros-rust-operating-system)
- [Table of Contents](#table-of-contents)
- [Building](#building)
  - [Cloning](#cloning)
  - [Compilation](#compilation)
  - [Running](#running)
    - [QEMU](#qemu)
    - [Hardware](#hardware)
- [License](#license)

# Building

## Cloning
You can clone the repository by running the following command:
```bash
$ git clone https://github.com/BastianAsmussen/ros
```

## Compilation
When building the OS, I recommend using [rustup](https://rustup.rs/), as it makes it easier to manage Rust versions and targets.
You can compile the OS by running the following command:
```bash
$ cargo build --release
```

## Running
You must have the `bootimage` crate installed to generate the bootable disk image, you can install it by running the following command:
```bash
$ cargo install bootimage
```

### QEMU
You can run the OS in [QEMU](https://www.qemu.org/) by running the following command:
```bash
$ cargo run
```
### Hardware
You can run the OS on real hardware by running the following commands:

#### Linux
```bash
$ cargo bootimage --release
$ dd if=target/x86_64-ros/release/bootimage-os.bin of=/dev/sdX && sync
```
Where `/dev/sdX` is the device name of your USB drive.

---

#### Windows
```ps
PS> cargo bootimage --release
```
Then use [Rufus](https://rufus.ie/) to flash the image to your USB drive.

# License
This project is licensed under the [MIT License](LICENSE).
