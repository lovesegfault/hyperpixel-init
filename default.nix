let
  pkgs = import ./nix;
in
pkgs.callPackage
  ({ naersk, cargo }:
    naersk.buildPackage {
      name = "hyperpixel_init";
      src = ./.;
      nativeBuildInputs = [ cargo ];
    })
{ }
