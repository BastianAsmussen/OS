# OS
My very own OS in Rust!

## Building
```bash
$ cargo build
```

## Testing
```bash
$ cargo test
```

## Running in QEMU
```bash
$ qemu-system-x86_64 -drive format=raw,file=target/x86_64-cristian_os/debug/bootimage-cristian_os.bin
```
