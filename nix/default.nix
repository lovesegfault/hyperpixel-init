let
  sources = import ./sources.nix;
  pkgs' = import sources.nixpkgs {
    overlays = [ (self: super: { naersk = self.callPackage sources.naersk { }; }) ];
  };
in
if builtins.currentSystem == "aarch64-linux" then pkgs' else pkgs'.pkgsCross.aarch64-multiplatform
