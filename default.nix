{ pkgs ? import ./nix }:
pkgs.callPackage
  (
    { rustPlatform }: rustPlatform.buildRustPackage {
      pname = "hyperpixel-init";
      version = "unstable";

      src = ./.;

      cargoSha256 = "1436gi1vb2jwi1zv15qp0qnh364qjwwvhd5x0mnk0fdx4ljv9wi7";
    }
  )
{ }
