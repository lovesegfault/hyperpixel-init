{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable-small";
  };

  outputs = { self, fenix, flake-utils, gitignore, naersk, nixpkgs, ... }:
    flake-utils.lib.eachDefaultSystem (localSystem:
      let
        crossSystem = nixpkgs.lib.systems.examples.aarch64-multiplatform-musl // { useLLVM = true; };

        pkgs = import nixpkgs {
          inherit localSystem crossSystem;
          overlays = [ fenix.overlay gitignore.overlay naersk.overlay ];
        };

        inherit (pkgs) pkgsBuildBuild pkgsBuildHost;

        llvmToolchain = pkgsBuildHost.llvmPackages_latest;

        rustToolchain = pkgsBuildHost.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-KXx+ID0y4mg2B3LHp7IyaiMrdexF6octADnAtFIOjrY=";
        };

        naerskCross = pkgsBuildHost.naersk.override {
          inherit (llvmToolchain) stdenv;
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        src = pkgs.gitignoreSource ./.;
      in
      {
        packages = {
          default = self.packages.${localSystem}.hyperpixel-init;
          hyperpixel-init = naerskCross.buildPackage {
            name = "hyperpixel-init";

            inherit src;

            nativeBuildInputs = with llvmToolchain; [ stdenv.cc lld ];
          };
        };

        devShells.default = pkgs.mkShell {
          name = "hyperpixel-init";

          inputsFrom = [ self.packages.${localSystem}.default ];

          nativeBuildInputs = with pkgsBuildBuild; [
            cargo-audit
            cargo-bloat
            cargo-edit
            cargo-udeps
            nix-linter
            nixpkgs-fmt
            pre-commit
            rnix-lsp
            rust-analyzer-nightly
            (pkgs.lib.lowPrio git)
          ];

          shellHook = ''
            pre-commit install --install-hooks
          '';
        };
      });
}
