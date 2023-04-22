# OS
My very own OS in Rust!

## Building
```bash
$ cargo build
$ cargo bootimage
```

## Running
Builds source and bootloader and runs it through QEMU.
```bash
$ cargo run
```

## QEMU
```bash
$ qemu-system-x86_64 -drive format=raw,file=target/x86_64-basic_os/debug/bootimage-basic_os.bin
```

## Testing
Runs all tests in the `tests` folder.
```bash
$ cargo test
```
