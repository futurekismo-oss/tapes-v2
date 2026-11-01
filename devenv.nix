{ pkgs, ... }: {
  languages.rust.enable = true;

  env.RUST_LOG = "trace";

  scripts.bump-version.exec = ''
    exec ${pkgs.zsh}/bin/zsh -c '
      if [ -z "$1" ] || [ -z "$2" ]; then
        echo "Usage: bump-version <old-version> <new-version>"
        exit 1
      fi
      old="''${1//./\\.}"
      git grep -l "$1" | xargs sed -i "s/$old/$2/g"
      echo "Successfully bumped version from $1 to $2 across the repo!"
    ' _ "$@"
  '';
}
