{ pkgs, ... }:

{
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [
      "rustfmt"
      "clippy"
      "rust-analyzer"
    ];
  };

  packages = [
    pkgs.cargo-audit
    pkgs.gcc
    pkgs.git
    pkgs.qemu
    pkgs.vim
  ];

  env.RUST_BACKTRACE = "1";

  tasks = {
    "vex:fmt" = {
      exec = "cargo fmt --all -- --check";
    };

    "vex:clippy" = {
      exec = "cargo clippy --all-targets -- -D warnings";
    };

    "vex:test" = {
      exec = "cargo test --locked --all-targets";
    };

    "vex:audit" = {
      exec = "cargo audit";
    };

    "vex:check" = {
      exec = "cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --locked --all-targets && cargo audit";
    };
  };
}
