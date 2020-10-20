let
  pkgs = import ./nix { system = "x86_64-linux"; };
in
pkgs.mkShell {
  name = "hyperpixel-init";
  nativeBuildInputs = with pkgs; [
    cargo
    cargo-edit
    niv
    nixpkgs-fmt
    rust-analyzer
  ];
}
