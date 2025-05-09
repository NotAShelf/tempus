{
  lib,
  rustPlatform,
}: let
  fs = lib.fileset;
in
  rustPlatform.buildRustPackage {
    pname = "tempus";
    version = "0.3.5";

    src = fs.toSource {
      root = ../.;
      fileset = fs.unions [
        (fs.fileFilter (file: builtins.any file.hasExt ["rs"]) ../src)
        ../Cargo.lock
        ../Cargo.toml
      ];
    };
    cargoLock.lockFile = ../Cargo.lock;
  }
