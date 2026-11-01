{...}: {
  languages.rust.enable = true;

  env.RUST_LOG="trace";

  enterShell = ''
    source <(COMPLETE=zsh tapes-v2)
  '';
}
