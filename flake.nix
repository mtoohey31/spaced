{
  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    mozillapkgs = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, naersk, mozillapkgs }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages."${system}";

        mozilla = pkgs.callPackage (mozillapkgs + "/package-set.nix") { };
        rustChannel = mozilla.rustChannelOf {
          date = "2022-04-28";
          channel = "nightly";
          sha256 = "m+Yg171wVTSr4Q04fe5KY3fL6RRmk855j/e0kqObW2M=";
        };
        rust = rustChannel.rust;

        naersk-lib = naersk.lib."${system}".override {
          cargo = rust;
          rustc = rust;
        };
      in
      rec {
        packages.spaced = naersk-lib.buildPackage {
          pname = "spaced";
          root = ./.;

          nativeBuildInputs = [ pkgs.sqlite ];
        };
        defaultPackage = packages.spaced;

        devShell = pkgs.mkShell {
          nativeBuildInputs = [ rust pkgs.sqlite pkgs.rust-analyzer pkgs.pandoc ];
          shellHook = ''
            export RUST_SRC_PATH="${rustChannel.rust-src}/lib/rustlib/src/rust/library"
          '';
        };
      });
}
