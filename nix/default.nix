{ system ? "aarch64-linux" }:
let
  sources = import ./sources.nix;
  lib = import (sources.nixpkgs + "/lib");
  pkgs = import sources.nixpkgs {
    inherit system;
    overlays = [(self: super: { naersk = self.callPackage sources.naersk { }; })];
  };
in
  pkgs
