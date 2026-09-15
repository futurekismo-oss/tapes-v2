{
  description = "Cassette tapes v2 dev shell (builds happen in CI)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
            git
            pkg-config
          ];
          shellHook = ''
            echo "tapes v2 dev shell — run: cargo fmt --check && cargo clippy --all-targets && cargo test"
          '';
        };
      }
    );
}
