# OS
My very own OS in Rust!

## Building
```bash
$ cargo build
$ cargo bootimage
```

## Testing
```bash
$ cargo test
```

## Running in QEMU
```bash
$ qemu-system-x86_64 -drive format=raw,file=target/x86_64-basic_os/debug/bootimage-basic_os.bin
```
