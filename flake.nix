{
  description = "Stockfin - A GTK4 stock portfolio tracker for Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    in {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = cargoToml.package.name;
        version = cargoToml.package.version;

        src = ./.;

        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
          wrapGAppsHook4
        ];

        buildInputs = with pkgs; [
          gtk4
          glib
          dbus
        ];

        meta = with pkgs.lib; {
          description = "A GTK4 stock portfolio tracker with Waybar integration";
          homepage = "https://github.com/jlodenius/stockfin";
          license = licenses.mit;
          maintainers = [
            {
              github = "jlodenius";
              name = "Jacob Lodenius";
            }
          ];
          platforms = platforms.linux;
        };
      };

      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rustc
          rust-analyzer
          pkg-config
          gtk4
          glib
          dbus
        ];
      };
    });
}
