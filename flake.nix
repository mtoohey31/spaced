{
  description = "spaced";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, utils, naersk }: {
    overlays = rec {
      expects-naersk = final: _: {
        spaced = final.naersk.buildPackage {
          pname = "spaced";
          root = builtins.path { path = ./.; name = "spaced-src"; };
          nativeBuildInputs = [ final.sqlite ];
        };
      };

      default = _: prev: {
        inherit (prev.appendOverlays [
          naersk.overlay
          expects-naersk
        ]) spaced;
      };
    };
  } // utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        overlays = [ self.overlays.default ];
        inherit system;
      };
      inherit (pkgs) cargo cargo-watch mkShell rust-analyzer rustc
        rustfmt spaced sqlite;
    in
    {
      packages.default = spaced;

      devShells.default = mkShell {
        packages = [
          cargo
          cargo-watch
          rust-analyzer
          rustc
          rustfmt
          sqlite
        ];
      };
    });
}
