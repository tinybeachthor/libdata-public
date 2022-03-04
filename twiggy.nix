{ lib
, rustPlatform
, fetchCrate
, nodejs
, pkg-config
, openssl
, stdenv
, curl
}:

rustPlatform.buildRustPackage rec {
  pname = "twiggy";
  version = "0.7.0";

  src = fetchCrate {
    inherit pname version;
    sha256 = "sha256-NbtS7A5Zl8634Q3xyjVzNraNszjt1uIXqmctArfnqkk=";
  };
  cargoSha256 = "sha256-94pfhVZ0CNMn+lCl5O+wOyE+D6fVXbH4NAPx92nMNbM=";

  nativeBuildInputs = [ pkg-config ];

  buildInputs = [ ];
  checkInputs = [ ];

  meta = with lib; {
    homepage = "https://rustwasm.github.io/twiggy/";
    license = with licenses; [ asl20 /* or */ mit ];
    description = "Twiggy is a code size profiler for Wasm.";
    mainProgram = "twiggy";
  };
}

