{
  description = "Simple HTTP(S) proxy for Github workflow triggers";

  inputs = {
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    crane,
    nixpkgs,
    flake-utils,
    ...
  } @ inputs:
    {
      nixosModules.default = import ./nix/module.nix self.overlays.default;

      overlays.default = final: prev: let
        workspaceCargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        pkgs = final.extend (import inputs.rust-overlay);
        craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;

          meta = with pkgs.lib; {
            inherit (workspaceCargoToml.package) description;
            mainProgram = "damascus";
            licenses = licenses.gpl2Only;
            maintainers = [maintainers.aftix];
            platforms = platforms.linux;
          };
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        commonArgsWithDeps = commonArgs // {inherit cargoArtifacts;};

        passthruChecks =
          {
            proxy-clippy = craneLib.cargoClippy (commonArgsWithDeps
              // {
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              });
            proxy-docs = craneLib.cargoDoc commonArgsWithDeps;
            proxy-fmt = craneLib.cargoFmt {inherit src;};
            proxy-toml-fmt = craneLib.taploFmt {
              src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];
            };
            proxy-audit = craneLib.cargoAudit (commonArgsWithDeps // {inherit (inputs) advisory-db;});
            proxy-deny = craneLib.cargoDeny commonArgsWithDeps;
            proxy-nextest = craneLib.cargoNextest (commonArgsWithDeps
              // {
                partitions = 1;
                partitionType = "count";
                cargoNextestPartitionsExtraArgs = "--no-tests=pass";
              });
            flake-statix = pkgs.callPackage ./nix/check-statix.nix {flakeSource = src;};
            flake-fmt = pkgs.callPackage ./nix/check-fmt.nix {flakeSource = src;};
            cargo-sort = pkgs.callPackage ./nix/check-sorted.nix {flakeSource = src;};
          }
          // pkgs.lib.attrsets.optionalAttrs (!pkgs.hostPlatform.isDarwin) {
            proxy-cov-llvm = craneLib.cargoLlvmCov commonArgsWithDeps;
          };

        devshell = craneLib.devShell {
          name = "actions-test-devshell";
          strictDeps = true;

          checks = passthruChecks;

          packages = with pkgs; [
            act
            alejandra
            cargo-deny
            cargo-nextest
            cargo-sort
            statix
            taplo
          ];
        };
      in {
        forgejo-gha-proxy = craneLib.buildPackage (commonArgsWithDeps
          // {
            passthru = {
              checks = passthruChecks;
              inherit devshell;
            };
            meta.mainProgram = "proxy";
          });
      };
    }
    // flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          self.overlays.default
        ];
      };

      inherit (pkgs) lib;
    in rec {
      apps.default = flake-utils.lib.mkApp {
        drv = packages.default;
      };

      inherit (packages.default.passthru) checks;

      packages = rec {
        inherit (pkgs) forgejo-gha-proxy;
        default = forgejo-gha-proxy;

        fullCheck = lib.pipe forgejo-gha-proxy.passthru.checks [
          (lib.flip lib.attrsets.mapAttrsToList)
          (lib.flip lib.id (_: value: value))
          (pkgs.linkFarmFromDrvs "full-check")
        ];
      };

      devShells.default = packages.default.passthru.devshell;
      formatter = pkgs.alejandra;
    });
}
