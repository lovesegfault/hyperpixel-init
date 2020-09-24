let
  pkgs = import ./nix;
in
pkgs.callPackage
  (
    { mkShell, cargo-edit, niv, nixpkgs-fmt, rustFull }:
    pkgs.mkShell {
      name = "hyperpixel-init";
      nativeBuildInputs = [
        cargo-edit
        niv
        nixpkgs-fmt
        rustFull
      ];
    }
  )
{ }
