{
  description = "The classic game of Pong, in your terminal, over ICMPv6!";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    (flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in rec
      {
        packages.icmpong = pkgs.rustPlatform.buildRustPackage {
          pname = "icmpong";
          version = "0.1.0";
          cargoLock.lockFile = ./Cargo.lock;
          src = pkgs.lib.cleanSource ./.;
        };
        defaultPackage = packages.icmpong;
      }
    ));
}
