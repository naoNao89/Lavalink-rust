{
  description = "Lavalink Rust - A standalone audio sending node for Discord";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Build inputs for Rust compilation
        buildInputs = with pkgs; [
          ffmpeg
          libopus
          alsa-lib
          openssl
          pkg-config
          python3
          python3Packages.yt-dlp
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustc
          cargo
        ];

      in
      {
        # Development shell
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          
          packages = with pkgs; [
            rust-analyzer
            rustfmt
            clippy
            cargo-watch
            curl
            jq
            nixpkgs-fmt
            ffmpeg
            python3Packages.yt-dlp
          ];
          
          shellHook = ''
            echo "🦀 Lavalink Rust Development Environment"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo ""
          '';
        };
      }
    ) // {
      # NixOS module
      nixosModules.default = import ./nix/module.nix;
    };
}
