{
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixos-unstable;
    rust-overlay.url = github:oxalica/rust-overlay/master;
    naersk.url = github:nmattia/naersk/master;
  };

  outputs = { self, nixpkgs, rust-overlay, naersk }:
    let
      supportedSystems = [ "x86_64-linux" ];

      # Function to generate a set based on supported systems
      forAllSystems = f:
        nixpkgs.lib.genAttrs supportedSystems (system: f system);

      nixpkgsFor = forAllSystems (system: import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlay
        ];
      });

    in {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" ];
            targets = [
              "x86_64-linux"
              "wasm32-unknown-unknown"
            ];
          };
          naersk-lib = naersk.lib."${system}".override {
            cargo = rust;
            rustc = rust;
          };
        in {
          src = pkgs.callPackage ./src.nix { };
          libdata-wasm = pkgs.callPackage ./libdata-wasm {
            libdata-src = pkgs.callPackage ./src.nix { };
            inherit naersk-lib;
          };
        });

      devShell = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rls-preview"
            ];
            targets = [
              "x86_64-unknown-linux-gnu"
              "wasm32-unknown-unknown"
            ];
          };
        in import ./shell.nix {
          inherit pkgs rust;
        });
    };
}
