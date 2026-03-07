{pkgs, ...}: {
  # https://devenv.sh/packages/
  packages = [pkgs.git];

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };
}
