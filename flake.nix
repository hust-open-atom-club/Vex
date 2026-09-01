{
  description = "Vex - a QEMU configuration manager";

  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    devenv.url = "github:cachix/devenv";
  };

  nixConfig = {
    extra-substituters = [ "https://devenv.cachix.org" ];
    extra-trusted-public-keys = [
      "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw="
    ];
  };

  outputs = { self, nixpkgs, devenv, ... }@inputs:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forEachSystem = nixpkgs.lib.genAttrs systems;

      mkDevenvShell = system:
        devenv.lib.mkShell {
          inherit inputs;
          pkgs = import nixpkgs {
            inherit system;
          };
          modules = [ ./devenv.nix ];
        };

      packageFor = system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        pkgs.rustPlatform.buildRustPackage {
          pname = "vex";
          version = "0.4.1";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          meta.mainProgram = "vex";
        };
    in
    {
      devShells = forEachSystem (system: {
        default = mkDevenvShell system;
      });

      packages = forEachSystem (system: {
        default = packageFor system;
      });

      apps = forEachSystem (system: {
        default = {
          type = "app";
          program = "${packageFor system}/bin/vex";
        };
      });
    };
}
