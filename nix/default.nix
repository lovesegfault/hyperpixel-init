let
  sources = import ./sources.nix;
  pkgs' = import sources.nixpkgs { };
in
if builtins.currentSystem == "aarch64-linux" then pkgs' else pkgs'.pkgsCross.aarch64-multiplatform
