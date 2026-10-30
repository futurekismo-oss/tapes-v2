{
  rustPlatform,
  lib,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "tapes-v2";
  version = "0.1.0-beta";

  src = ../.;

  cargoHash = "sha256-m5WbWwT9veVZSH6/sSruClF927OEPGQe/IWVSG4gW28=";

  meta = {
    description = "Swap your whole rice in one command. Eject it with one more. Atomic dotfiles switching for Linux.";
    homepage = "https://github.com/futurekismo-oss/tapes-v2";
    lincese = with lib.licenses; [
      mit
    ];
    maintainers = with lib.maintainers; [
      futurekismo
    ];
    mainProgram = "tapes-v2";
    platforms = [ "x86_64-linux" ];
  };
})
