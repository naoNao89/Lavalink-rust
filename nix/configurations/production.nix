# Production environment configuration
{ config, pkgs, lib, ... }:

{
  imports = [
    ../module.nix
  ];

  # Basic system configuration
  system.stateVersion = "23.11";
  
  # Enable Lavalink Rust service
  services.lavalink-rust = {
    enable = true;
    openFirewall = true;
    
    settings = {
      server = {
        port = 2333;
        address = "0.0.0.0";
      };
      
      lavalink = {
        server = {
          # Use environment variable or secrets management
          password = "$LAVALINK_PASSWORD";
          sources = {
            youtube = true;
            bandcamp = true;
            soundcloud = true;
            twitch = true;
            vimeo = true;
            http = true;
            local = false;  # Disable local files in production
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
    
    environment = {
      RUST_LOG = "info";
      RUST_BACKTRACE = "1";
    };
  };

  # Production system packages (minimal)
  environment.systemPackages = with pkgs; [
    curl
    htop
    
    # Essential audio tools
    ffmpeg
    python3Packages.yt-dlp
  ];

  # Secure SSH configuration
  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = false;
      PermitRootLogin = "no";
      PubkeyAuthentication = true;
    };
  };

  # Production monitoring with Prometheus
  services.prometheus = {
    enable = true;
    port = 9091;
    
    # Restrict access to localhost only
    listenAddress = "127.0.0.1";
    
    scrapeConfigs = [
      {
        job_name = "lavalink-rust";
        static_configs = [
          {
            targets = [ "localhost:9090" ];
          }
        ];
        scrape_interval = "30s";
      }
      {
        job_name = "node-exporter";
        static_configs = [
          {
            targets = [ "localhost:9100" ];
          }
        ];
        scrape_interval = "30s";
      }
    ];
    
    # Retention policy
    retentionTime = "30d";
  };

  # Node exporter for system metrics
  services.prometheus.exporters.node = {
    enable = true;
    enabledCollectors = [
      "systemd"
      "processes"
      "cpu"
      "meminfo"
      "diskstats"
      "filesystem"
      "loadavg"
      "netdev"
    ];
  };

  # Grafana for visualization
  services.grafana = {
    enable = true;
    settings = {
      server = {
        http_addr = "127.0.0.1";  # Only localhost access
        http_port = 3000;
        domain = "lavalink.example.com";
        root_url = "https://lavalink.example.com/grafana/";
      };
      security = {
        admin_password = "$__file{/etc/grafana-admin-password}";
        secret_key = "$__file{/etc/grafana-secret-key}";
      };
      analytics = {
        reporting_enabled = false;
      };
    };
  };

  # Reverse proxy with nginx
  services.nginx = {
    enable = true;
    
    virtualHosts."lavalink.example.com" = {
      enableACME = true;
      forceSSL = true;
      
      locations = {
        "/" = {
          proxyPass = "http://127.0.0.1:2333";
          proxyWebsockets = true;
          extraConfig = ''
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
          '';
        };
        
        "/grafana/" = {
          proxyPass = "http://127.0.0.1:3000/";
          extraConfig = ''
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
          '';
        };
      };
    };
  };

  # ACME certificates
  security.acme = {
    acceptTerms = true;
    defaults.email = "admin@example.com";
  };

  # Firewall configuration (restrictive)
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 80 443 ];
    
    # Allow internal monitoring
    interfaces.lo.allowedTCPPorts = [ 2333 9090 9091 9100 3000 ];
  };

  # System hardening
  security = {
    # Disable sudo for regular users
    sudo.enable = false;
    
    # Enable fail2ban
    fail2ban = {
      enable = true;
      jails = {
        ssh.settings = {
          enabled = true;
          port = "ssh";
          filter = "sshd";
          logpath = "/var/log/auth.log";
          maxretry = 3;
          bantime = 3600;
        };
      };
    };
  };

  # Automatic updates
  system.autoUpgrade = {
    enable = true;
    flake = "github:lavalink-devs/lavalink-rust";
    flags = [
      "--update-input"
      "nixpkgs"
      "--commit-lock-file"
    ];
    dates = "04:00";
    randomizedDelaySec = "45min";
  };

  # Log rotation and management
  services.logrotate = {
    enable = true;
    settings = {
      "/var/log/lavalink-rust/*.log" = {
        frequency = "daily";
        rotate = 30;
        compress = true;
        delaycompress = true;
        missingok = true;
        notifempty = true;
      };
    };
  };

  # Backup configuration (example)
  services.restic.backups.lavalink = {
    initialize = true;
    repository = "s3:backup-bucket/lavalink-rust";
    passwordFile = "/etc/restic-password";
    
    paths = [
      "/var/lib/lavalink-rust"
      "/etc/nixos"
    ];
    
    timerConfig = {
      OnCalendar = "daily";
      RandomizedDelaySec = "1h";
    };
    
    pruneOpts = [
      "--keep-daily 7"
      "--keep-weekly 4"
      "--keep-monthly 12"
    ];
  };
}
