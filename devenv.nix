{pkgs, ...}: {
  packages = with pkgs; [git cargo-bootimage qemu];

  languages.rust = {
    enable = true;
    channel = "nightly";
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" "rust-src" "rust-std" "llvm-tools"];
    targets = ["x86_64-unknown-none"];
  };

  pre-commit.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
  };
}
