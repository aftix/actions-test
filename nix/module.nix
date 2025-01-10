overlay: {
  pkgs,
  lib,
  config,
  ...
}: let
  inherit (lib.options) mkEnableOption mkOption mkPackageOption;

  cfg = config.services.forgejo-gha-proxy;
in {
  options.services.forgejo-gha-proxy = {
    enable = mkEnableOption "forgejo gha proxy";

    package = mkPackageOption pkgs "forgejo-gha-proxy" {
      default = ["forgejo-gha-proxy"];
    };

    port = mkOption {
      default = 9988;
      type = lib.types.ints.positive;
    };

    tls = mkOption {
      default = true;
      type = lib.types.bool;
    };
  };

  config = lib.mkIf cfg.enable {
    nixpkgs.overlays = [overlay];

    systemd = {
      services.forgejo-gha-proxy = let
        workspaceCargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      in {
        inherit (workspaceCargoToml.package) description;

        path = [pkgs.forgejo-gha-proxy];
        environment = {
          LISTEN_PORT = cfg.port;
          # TODO: setup tls
        };

        script = ''
          forgejo-gha-proxy
        '';

        serviceConfig = {
          Type = "simple";
          RestartSec = "5min";

          User = "forgejo-gha-proxy";
          Group = "forgejo-gha-proxy";
          DynamicUser = true;

          IPAddressAllow = "localhost";
          IPAddressDeny = "any";
          LockPersonality = true;
          MemoryDenyWriteExecute = true;
          NoNewPrivileges = true;
          PrivateDevices = true;
          PrivateTmp = true;
          PrivateUsers = true;
          ProtectHome = "read-only";
          ProtectHostname = true;
          ProtectKernelModules = true;
          ProtectKernelTunables = true;
          ProtectProc = "invisible";
          ProtectSystem = "strict";
          RemoveIPC = true;
          RestrictNamespaces = "";
          RestrictRealtime = true;
          RestrictSUIDSGID = true;
          SystemCallArchitectures = "native";
          UMask = "0027";
        };
      };
    };
  };
}
