{
  rustPlatform,
  lib,
  pkgs,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "tapes";
  version = "0.1.6-beta";

  src = ../.;

  nativeBuildInputs = [
   pkgs.mold
  ];

  cargoHash = "sha256-i2cT0BILjcK9WqqKUoq2BWVDFbO9ujLkGRiJAnkCoRU=";

  env.RUSTC_WRAPPER = "";

  meta = {
    description = "Swap your whole rice in one command. Eject it with one more. Atomic dotfiles switching for Linux.";
    homepage = "https://github.com/futurekismo-oss/tapes";
    license = with lib.licenses; [
      mit
    ];
    maintainers = with lib.maintainers; [
      futurekismo
    ];
    mainProgram = "tapes";
    platforms = [ "x86_64-linux" ];
  };
})
