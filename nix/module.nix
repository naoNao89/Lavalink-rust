{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.lavalink-rust;
  
  # Default configuration
  defaultConfig = {
    server = {
      port = 2333;
      address = "0.0.0.0";
    };
    lavalink = {
      server = {
        password = "youshallnotpass";
        sources = {
          youtube = true;
          bandcamp = true;
          soundcloud = true;
          twitch = true;
          vimeo = true;
          http = true;
          local = false;
        };
      };
    };
    logging = {
      level = {
        root = "INFO";
        lavalink = "INFO";
      };
    };
  };

  # Configuration file
  configFile = pkgs.writeText "application.yml" (
    builtins.toJSON (recursiveUpdate defaultConfig cfg.settings)
  );

in {
  options.services.lavalink-rust = {
    enable = mkEnableOption "Lavalink Rust audio server";

    package = mkOption {
      type = types.package;
      default = pkgs.lavalink-rust;
      description = "The Lavalink Rust package to use";
    };

    user = mkOption {
      type = types.str;
      default = "lavalink";
      description = "User account under which Lavalink runs";
    };

    group = mkOption {
      type = types.str;
      default = "lavalink";
      description = "Group under which Lavalink runs";
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/lavalink-rust";
      description = "Directory where Lavalink stores its data";
    };

    configFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to the configuration file. If null, a default configuration will be generated.";
    };

    settings = mkOption {
      type = types.attrs;
      default = {};
      description = "Configuration settings for Lavalink. These will be merged with the default configuration.";
      example = literalExpression ''
        {
          server.port = 2334;
          lavalink.server.password = "supersecret";
        }
      '';
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Whether to open the firewall for Lavalink";
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Extra command line arguments to pass to Lavalink";
    };

    environment = mkOption {
      type = types.attrsOf types.str;
      default = {};
      description = "Environment variables to set for the Lavalink process";
      example = literalExpression ''
        {
          RUST_LOG = "debug";
          RUST_BACKTRACE = "1";
        }
      '';
    };

    jvmOptions = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "JVM options (kept for compatibility, not used in Rust version)";
    };
  };

  config = mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home = cfg.dataDir;
      createHome = true;
      description = "Lavalink Rust service user";
    };

    users.groups.${cfg.group} = {};

    systemd.services.lavalink-rust = {
      description = "Lavalink Rust Audio Server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = cfg.dataDir;
        
        ExecStart = "${cfg.package}/bin/lavalink-rust --config ${
          if cfg.configFile != null then cfg.configFile else configFile
        } ${concatStringsSep " " cfg.extraArgs}";
        
        Restart = "always";
        RestartSec = "10s";
        
        # Security settings
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ cfg.dataDir ];
        
        # Resource limits
        LimitNOFILE = 65536;
        
        # Environment
        Environment = mapAttrsToList (name: value: "${name}=${value}") (
          {
            RUST_LOG = "info";
            RUST_BACKTRACE = "1";
          } // cfg.environment
        );
      };

      preStart = ''
        # Ensure data directory exists and has correct permissions
        mkdir -p ${cfg.dataDir}/{logs,plugins}
        chown -R ${cfg.user}:${cfg.group} ${cfg.dataDir}
        chmod 755 ${cfg.dataDir}
        chmod 750 ${cfg.dataDir}/{logs,plugins}
      '';
    };

    # Firewall configuration
    networking.firewall = mkIf cfg.openFirewall {
      allowedTCPPorts = [ 
        (cfg.settings.server.port or defaultConfig.server.port)
        9090  # Metrics port
      ];
    };

    # Ensure required packages are available
    environment.systemPackages = with pkgs; [
      cfg.package
      python3Packages.yt-dlp
      ffmpeg
    ];

    # Log rotation
    services.logrotate.settings.lavalink-rust = {
      files = "${cfg.dataDir}/logs/*.log";
      frequency = "daily";
      rotate = 7;
      compress = true;
      delaycompress = true;
      missingok = true;
      notifempty = true;
      create = "644 ${cfg.user} ${cfg.group}";
    };
  };
}
