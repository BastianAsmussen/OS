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
```sh
$ git clone https://github.com/BastianAsmussen/ROS.git
```

## Compilation
When building the OS, I recommend using [devenv](https://devenv.sh/).
You can compile the OS by running the following command:
```sh
$ cargo build --release
```

## Running

You must have the `bootimage` crate installed to generate the bootable disk image, if using [devenv](https://devenv.sh), you don't need to do anything else.  
Otherwise, you can install it by running the following command:
```sh
$ cargo install bootimage
```

### QEMU
You can run the OS in [QEMU](https://www.qemu.org/) by running the following command:
```sh
$ cargo run
```

### Hardware
You can run the OS on real hardware by running the following commands:

#### Linux
```sh
$ cargo bootimage --release
# dd if=target/x86_64-ros/release/bootimage-os.bin of=/dev/sdX && sync
```
Where `/dev/sdX` is the device name of your USB drive.

