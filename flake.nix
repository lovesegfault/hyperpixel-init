{
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.utils.follows = "utils";
      inputs.flake-compat.follows = "flake-compat";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable-small";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
    rust = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
  };

  outputs = { self, crane, gitignore, nixpkgs, pre-commit-hooks, rust, utils, ... }:
    utils.lib.eachDefaultSystem (localSystem:
      let
        crossSystem = "aarch64-linux";
        pkgs = import nixpkgs {
          inherit localSystem crossSystem;
          overlays = [
            rust.overlay
            gitignore.overlay
          ];
        };

        rustToolchain = pkgs.pkgsBuildHost.rust-bin.stable.latest.default;

        craneLib = (crane.mkLib pkgs).overrideScope' (final: prev: {
          cargo = rustToolchain;
          rustc = rustToolchain;
          clippy = rustToolchain;
          rustfmt = rustToolchain;
        });

        src = pkgs.gitignoreSource ./.;

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
        };

        crateFmt = craneLib.cargoFmt {
          inherit cargoArtifacts src;
        };

        crateClippy = craneLib.cargoClippy {
          inherit src;
          cargoArtifacts = crateFmt;
          cargoClippyExtraArgs = "-- --deny warnings";
        };

        crate = craneLib.buildPackage {
          inherit src;
          cargoArtifacts = crateClippy;
        };
      in
      {
        packages.hyperpixel-init = crate;

        defaultPackage = self.packages.${localSystem}.hyperpixel-init;

        devShell = pkgs.mkShell {
          name = "hyperpixel-init";

          inputsFrom = [ self.defaultPackage.${localSystem} ];

          nativeBuildInputs = with pkgs.pkgsBuildBuild; [
            cargo-audit
            cargo-bloat
            cargo-edit
            cargo-udeps
            nix-linter
            nixpkgs-fmt
            rnix-lsp
            rust-analyzer
          ];
          inherit (self.checks.${localSystem}.pre-commit-check) shellHook;
        };

        checks.pre-commit-check = (pre-commit-hooks.lib.${localSystem}.run {
          inherit src;
          hooks = {
            nix-linter.enable = true;
            nixpkgs-fmt.enable = true;
          };
        });
      });
}
