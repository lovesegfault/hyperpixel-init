let
  sources = import ./sources.nix;
  overlays = [
    (import sources.nixpkgs-mozilla)
    (self: super:
      {
        rustChannel = self.rustChannelOf { channel = "nightly"; };
        rustFull = self.rustChannel.rust.override {
          extensions = [
            "clippy-preview"
            "rust-analysis"
            "rust-analyzer-preview"
            "rls-preview"
            "rust-src"
            "rust-std"
            "rustfmt-preview"
          ];
        };
      }
    )
    (self: super: { naersk = self.callPackage sources.naersk { }; })
  ];
in
import sources.nixpkgs { inherit overlays; system = "aarch64-linux"; }
