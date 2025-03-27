{
  shell-name,
  system,
  inputs,
  version ? "stable",
  profile ? "default",
  packages ? [],
  lib,
  ...
}: let
  # apply the rust overlay to nixpkgs
  pkgs = import inputs.nixpkgs {
    inherit system;
    overlays = [(import inputs.rust-overlay)];
  };

  # get rust binary version and profile
  rust-bin = pkgs.rust-bin.${version}.latest.${profile}.override {
    extensions = ["rust-src"];
  };
in
  pkgs.mkShell rec {
    name = "${shell-name}";
    nativeBuildInputs = [
      pkgs.pkg-config
    ];
    buildInputs =
      packages
      ++ [rust-bin]
      ++ (with pkgs; [
        pkg-config
        alsa-lib
        vulkan-tools
        vulkan-headers
        vulkan-loader
        vulkan-validation-layers
        libxkbcommon
        libinput
        libgbm
        libGL
        pixman
        wayland
        systemd
        seatd
        udev
        clang
        lld

        cargo-expand
        rust-analyzer
        weston
        just
      ]);

    RUST_SRC_PATH = "${rust-bin}/lib/rustlib/src/rust/library";
    LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
  }
