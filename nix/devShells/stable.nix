{
  pkgs,
  inputs,
  ...
}:
pkgs.callPackage ./build.nix {
  inherit inputs;
  shell-name = "rust-stable";
}
