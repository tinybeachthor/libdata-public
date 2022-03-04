{ stdenv
, lib
}:

stdenv.mkDerivation {
  name = "libdata-src";
  src = ./.;

  phases = [ "unpackPhase" "installPhase" ];

  installPhase =
    let
      members = (lib.importTOML ./Cargo.toml).workspace.members;
      member_to_mv = m: "mv ${m} $out;";
    in
    ''
      mkdir $out
      mv Cargo.toml $out
      mv Cargo.lock $out
      ${lib.concatMapStrings member_to_mv members}
    '';
}
