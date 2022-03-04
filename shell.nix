{ pkgs, rust }:

with pkgs;

let
  xetex = texlive.combine {
    inherit (texlive) scheme-small;
  };
  wasm-bindgen-cli = pkgs.callPackage ./wasm-bindgen-cli.nix {
    inherit (pkgs.stdenv.darwin.apple_sdk.frameworks) Security;
  };
  twiggy = pkgs.callPackage ./twiggy.nix { };

in
mkShell {
  buildInputs = [
    git
    hub
    gnumake
    pkg-config
    openssl

    rust
    cargo-tarpaulin
    cargo-insta

    protobuf

    wasm-bindgen-cli
    binaryen
    twiggy
    nodejs
    geckodriver

    xetex
  ];

  PROTOC="${protobuf}/bin/protoc";
  GECKODRIVER="${geckodriver}/bin/geckodriver";
}
