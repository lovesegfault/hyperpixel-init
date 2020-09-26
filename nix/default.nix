{ system ? "aarch64-linux" }:
let
  sources = import ./sources.nix;
  lib = import (sources.nixpkgs + "/lib");
  pkgs = import sources.nixpkgs {
    inherit system;
    overlays = [
      (import sources.nixpkgs-mozilla)
      (self: super: {
        rustChannel = self.rustChannelOf { channel = "stable"; };
        rustFull = self.rustChannel.rust.override {
          extensions = [
            "clippy-preview"
            "rust-analysis"
            # "rust-analyzer-preview"
            "rls-preview"
            "rust-src"
            "rust-std"
            "rustfmt-preview"
          ];
        };
        cargo = self.rustChannel.rust;
        rustc = self.rustChannel.rust;
      })
      (self: super: { naersk = self.callPackage sources.naersk { }; })
    ];
  };
  # // lib.optionalAttrs (builtins.currentSystem != "aarch64-linux") { crossSystem = lib.systems.examples.aarch64-multiplatform; };
in
pkgs
