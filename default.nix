{ pkgs ? import ./nix }:
let
    hyperpixel_init = { naersk, lib }: naersk.buildPackage {
        name = "hyperpixel_init";
        src = lib.cleanSource ./.;
    };
in
pkgs.callPackage hyperpixel_init { }
