let
  pkgs = import ./nix { system = builtins.currentSystem; };
in
pkgs.mkShell {
  name = "hyperpixel-init";
  nativeBuildInputs = with pkgs; [
    cargo-edit
    niv
    nixpkgs-fmt
    rustFull
  ];
}
