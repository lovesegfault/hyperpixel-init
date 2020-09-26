{ pkgs ? import ./nix { } }:
let
  hyperpixel_init = { naersk, lib }: naersk.buildPackage {
    name = "hyperpixel_init";
    src = lib.cleanSource ./.;
    CARGO_BUILD_TARGET = "aarch64-unknown-linux-gnu";
  };
in
pkgs.callPackage hyperpixel_init { }
