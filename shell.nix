let
  pkgs = import ./nix;
in
pkgs.mkShell {
  name = "hyperpixel-init";
  buildInputs = with pkgs; [
    cargo-edit
    niv
    nixpkgs-fmt
    rustFull
  ];
}
