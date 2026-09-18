{
  description = "Optional Recall Gate development shell (mirrors docs/developing.md system packages)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            sqlite
            gtk4
            gtk4-layer-shell
            wayland
            libxkbcommon
            xorg.libX11
            xorg.libXrandr
          ];
        };

        packages.recallgate-core = rustPlatform.buildRustPackage {
          pname = "recallgate-core";
          version = "0.0.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "-p" "recallgate-core" ];
          doCheck = true;
          checkFlags = [ "-p" "recallgate-core" ];
        };
      });
}
