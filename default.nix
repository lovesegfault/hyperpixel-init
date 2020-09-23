let
  pkgs = import ./nix;
in
pkgs.callPackage
  ({ naersk, cargo, gcc, pkgconfig, raspberrypi-tools, llvmPackages }:
    naersk.buildPackage {
      name = "hyperpixel_init";
      src = ./.;
      nativeBuildInputs = [ cargo gcc pkgconfig ];
      buildInputs = [ raspberrypi-tools ];
      LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
    })
{ }
