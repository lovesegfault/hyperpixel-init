{
  inputs = {
    fenix = {
      url = "github:figsoda/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, fenix, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system: {
      defaultPackage =
        let
          pkgs = nixpkgs.legacyPackages.${system};
          target = "aarch64-unknown-linux-gnu";
          toolchain = with fenix.packages.${system}; combine [
            minimal.rustc
            minimal.cargo
            targets.${target}.latest.rust-std
          ];
          naerskCross = naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          };
        in
        naerskCross.buildPackage {
          src = ./.;
          CARGO_BUILD_TARGET = target;
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.pkgsCross.aarch64-multiplatform.stdenv.cc}/bin/${target}-gcc";
        };
    });
}
