{pkgs, ...}: {
  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    tombi
  ];

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };
}
