{
  rustPlatform,
  lib,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "tapes-v2";
  version = "0.1.0-beta";

  src = ../.;

  cargoHash = "sha256-m5WbWwT9veVZSH6/sSruClF927OEPGQe/IWVSG4gW28=";
})
