# Development environment configuration
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
          password = "dev-password";
          sources = {
            youtube = true;
            bandcamp = true;
            soundcloud = true;
            twitch = true;
            vimeo = true;
            http = true;
            local = true;  # Enable local files for development
          };
        };
      };
      
      logging = {
        level = {
          root = "DEBUG";
          lavalink = "DEBUG";
        };
      };
    };
    
    environment = {
      RUST_LOG = "debug";
      RUST_BACKTRACE = "full";
    };
  };

  # Development tools
  environment.systemPackages = with pkgs; [
    curl
    jq
    htop
    git
    vim
    tmux
    
    # Audio tools for testing
    ffmpeg
    python3Packages.yt-dlp
    
    # Monitoring tools
    prometheus
    grafana
  ];

  # Enable SSH for remote development
  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = true;
      PermitRootLogin = "yes";
    };
  };

  # Prometheus monitoring (optional for development)
  services.prometheus = {
    enable = true;
    port = 9091;
    
    scrapeConfigs = [
      {
        job_name = "lavalink-rust";
        static_configs = [
          {
            targets = [ "localhost:9090" ];
          }
        ];
        scrape_interval = "15s";
      }
    ];
  };

  # Grafana for visualization (optional for development)
  services.grafana = {
    enable = true;
    settings = {
      server = {
        http_addr = "0.0.0.0";
        http_port = 3000;
      };
      security = {
        admin_password = "admin";
      };
    };
  };

  # Open additional ports for development
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 2333 9090 9091 3000 ];
  };

  # Enable systemd journal for debugging
  services.journald.extraConfig = ''
    SystemMaxUse=1G
    MaxRetentionSec=7day
  '';
}
