let
  pkgs' = import ./nix;
  pkgs = if builtins.currentSystem == "aarch64-linux"
    then pkgs'
    else pkgs'.pkgsCross.aarch64-multiplatform
  ;
in
pkgs.callPackage
  ({ naersk, cargo, gitignoreSource }:
    naersk.buildPackage {
      name = "hyperpixel_init";
      src = ./.;
      nativeBuildInputs = [ cargo ];
    })
{ }
