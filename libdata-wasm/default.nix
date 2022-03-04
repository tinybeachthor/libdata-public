{ libdata-src
, naersk-lib
, protobuf
, stdenv
, wasm-bindgen-cli
}:

let
  libdata-wasm-crate = naersk-lib.buildPackage {
    name = "libdata-wasm-crate";
    version = "latest";

    src = libdata-src.src;

    release = true;

    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";

    cargoBuildOptions = x: x ++ [
      "-p libdata-wasm"
      "--target wasm32-unknown-unknown"
    ];

    copyLibs = true;
    doCheck = false;

    buildInputs = [
      protobuf
    ];
    PROTOC="${protobuf}/bin/protoc";
  };

in
  stdenv.mkDerivation {
    pname = "libdata-wasm";
    version = "latest";

    src = libdata-wasm-crate;

    buildInputs = [
      wasm-bindgen-cli
    ];

    buildPhase = ''
      mkdir -p $out/dev
      wasm-bindgen --target bundler --out-dir $out/dev lib/libdata_wasm.wasm
    '';
    dontInstall = true;
  }
