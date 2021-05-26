{
  inputs = {
    fenix = {
      url = "github:figsoda/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.naersk.follows = "naersk";
    };
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, fenix, flake-utils, naersk, nixpkgs, pre-commit-hooks }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        fenixPkgs = fenix.packages.${system};
        target = "aarch64-unknown-linux-gnu";
        rustFull = with fenixPkgs; combine [
          (stable.withComponents [
            "cargo"
            "clippy-preview"
            "rust-src"
            "rust-std"
            "rustc"
            "rustfmt-preview"
          ])
          targets.${target}.stable.rust-std
        ];
        rustMinimal = with fenixPkgs; combine [
          (minimal.withComponents [
            "cargo"
            "rust-std"
            "rustc"
          ])
          targets.${target}.minimal.rust-std
        ];

        naerskBuild = (naersk.lib.${system}.override {
          cargo = rustMinimal;
          rustc = rustMinimal;
        }).buildPackage;

        cargoConfig = {
          CARGO_BUILD_TARGET = target;
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER =
            if "${system}" == "aarch64-linux" then
              "${pkgs.stdenv.cc}/bin/gcc"
            else
              "${pkgs.pkgsCross.aarch64-multiplatform.stdenv.cc}/bin/${target}-gcc";
        };
      in
      {
        packages.hyperpixel-init = naerskBuild ({
          src = ./.;
          doDoc = true;
        } // cargoConfig);

        defaultPackage = self.packages.${system}.hyperpixel-init;

        devShell = pkgs.mkShell ({
          inherit (self.checks.${system}.pre-commit-check) shellHook;
          name = "hyperpixel-init";
          buildInputs = with pkgs; [
            cargo-edit
            fenixPkgs.rust-analyzer
            nix-linter
            nixpkgs-fmt
            rustFull
          ];
        } // cargoConfig);

        checks = {
          pre-commit-check = (pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              nixpkgs-fmt.enable = true;
              nix-linter.enable = true;
              rustfmt = {
                enable = true;
                entry = pkgs.lib.mkForce ''
                  bash -c 'PATH="$PATH:${rustFull}/bin" cargo fmt -- --check --color always'
                '';
              };
            };
          });
        };
      });
}
