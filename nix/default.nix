let
  sources = import ./sources.nix;
  lib = import (sources.nixpkgs + "/lib");
  pkgs = import sources.nixpkgs {
    overlays = [ (self: super: { naersk = self.callPackage sources.naersk { }; }) ];
    system = "aarch64-linux";
  };
  # // lib.optionalAttrs (builtins.currentSystem != "aarch64-linux") { crossSystem = lib.systems.examples.aarch64-multiplatform; };
in
pkgs
