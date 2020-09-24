let
  pkgs' = import ./nix;
  pkgs = import pkgs'.path { };
in
pkgs.mkShell {
  name = "hyperpixel-init";
  nativeBuildInputs = with pkgs; [
    cargo-edit
    niv
    nixpkgs-fmt
    cargo
    rust-analyzer
  ];
}
