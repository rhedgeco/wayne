{
  system,
  inputs,
  ...
}: let
  pkgs = import inputs.nixpkgs {
    inherit system;
    config.allowUnfree = true;
  };
in
  pkgs.callPackage ./build.nix {
    inherit inputs;
    shell-name = "rust-stable-vscode";

    # include vscode with all necessary extensions
    packages = [
      (pkgs.vscode-with-extensions.override {
        vscodeExtensions = with pkgs.vscode-extensions;
          [
            jnoortheen.nix-ide
            kamadorueda.alejandra
            rust-lang.rust-analyzer
            tamasfe.even-better-toml
            vadimcn.vscode-lldb
            nefrob.vscode-just-syntax
          ]
          ++ (with inputs.vscode-extensions.extensions.${system}.vscode-marketplace; [
            citreae535.sparse-crates
          ]);
      })
    ];
  }
