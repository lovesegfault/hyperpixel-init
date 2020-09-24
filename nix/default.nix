let
  sources = import ./sources.nix;
  overlays = [
    (import sources.nixpkgs-mozilla)
    (self: super:
      {
        rustChannel = self.rustChannelOf { channel = "nightly"; };
        rustFull = self.rustChannel.rust.override {
          targets = [ "aarch64-unknown-linux-gnu" ];
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
        cargo = self.rustFull;
        rustc = self.rustFull;
      }
    )
    (self: super: { gitignoreSource = import sources.gitignoreSource; })
    (self: super: { naersk = self.callPackage sources.naersk { }; })
  ];
in
  import sources.nixpkgs { inherit overlays; }
