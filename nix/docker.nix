{ pkgs, lavalink-rust }:

let
  # Create a minimal user for the container
  user = "lavalink";
  uid = "1000";
  gid = "1000";

  # Create application configuration
  defaultConfig = pkgs.writeText "application.yml" ''
    server:
      port: 2333
      address: 0.0.0.0

    lavalink:
      server:
        password: "youshallnotpass"
        sources:
          youtube: true
          bandcamp: true
          soundcloud: true
          twitch: true
          vimeo: true
          http: true
          local: false

    logging:
      level:
        root: INFO
        lavalink: INFO
  '';

  # Create entrypoint script
  entrypoint = pkgs.writeShellScript "entrypoint.sh" ''
    #!/bin/bash
    set -euo pipefail

    # Create necessary directories
    mkdir -p /app/{logs,plugins}

    # Set proper ownership if running as root
    if [ "$(id -u)" = "0" ]; then
        chown -R ${uid}:${gid} /app
        exec gosu ${uid}:${gid} "$@"
    fi

    # Default configuration if none provided
    if [ ! -f /app/application.yml ]; then
        cp ${defaultConfig} /app/application.yml
    fi

    # Execute the main command
    exec "$@"
  '';

in {
  # Standard Docker image
  standard = pkgs.dockerTools.buildLayeredImage {
    name = "lavalink-rust";
    tag = "latest";
    
    contents = with pkgs; [
      # Main application
      lavalink-rust
      
      # System essentials
      bashInteractive
      coreutils
      findutils
      gnugrep
      gnused
      gawk
      curl
      cacert
      
      # Audio dependencies
      ffmpeg
      libopus
      python3Packages.yt-dlp
      
      # User management
      shadow
      gosu
    ];

    fakeRootCommands = ''
      # Create user and group
      ${pkgs.shadow}/bin/groupadd -g ${gid} ${user}
      ${pkgs.shadow}/bin/useradd -u ${uid} -g ${gid} -d /app -s /bin/bash ${user}
      
      # Create directories
      mkdir -p /app/{logs,plugins}
      chown -R ${uid}:${gid} /app
      chmod 755 /app
      chmod 750 /app/{logs,plugins}
    '';

    config = {
      Cmd = [ "${lavalink-rust}/bin/lavalink-rust" "--config" "/app/application.yml" ];
      Entrypoint = [ "${entrypoint}" ];
      
      ExposedPorts = {
        "2333/tcp" = {};
        "9090/tcp" = {};
      };
      
      Env = [
        "PATH=/bin:${pkgs.coreutils}/bin:${pkgs.curl}/bin:${pkgs.ffmpeg}/bin"
        "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        "RUST_LOG=info"
        "RUST_BACKTRACE=1"
      ];
      
      WorkingDir = "/app";
      User = "${uid}:${gid}";
      
      Labels = {
        "org.opencontainers.image.title" = "Lavalink Rust";
        "org.opencontainers.image.description" = "A standalone audio sending node for Discord, written in Rust";
        "org.opencontainers.image.source" = "https://github.com/lavalink-devs/lavalink-rust";
        "org.opencontainers.image.licenses" = "MIT";
        "org.opencontainers.image.version" = "4.0.0";
      };
    };
  };

  # Alpine-style minimal image
  minimal = pkgs.dockerTools.buildLayeredImage {
    name = "lavalink-rust";
    tag = "minimal";
    
    contents = with pkgs; [
      lavalink-rust
      cacert
      python3Packages.yt-dlp
      ffmpeg-headless
      libopus
      curl
      busybox
    ];

    config = {
      Cmd = [ "${lavalink-rust}/bin/lavalink-rust" ];
      ExposedPorts = {
        "2333/tcp" = {};
        "9090/tcp" = {};
      };
      Env = [
        "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        "RUST_LOG=info"
      ];
      WorkingDir = "/app";
      User = "1000:1000";
    };
  };

  # Debug image with additional tools
  debug = pkgs.dockerTools.buildLayeredImage {
    name = "lavalink-rust";
    tag = "debug";
    
    contents = with pkgs; [
      # Main application
      lavalink-rust
      
      # System tools
      bashInteractive
      coreutils
      findutils
      gnugrep
      gnused
      gawk
      curl
      cacert
      
      # Audio dependencies
      ffmpeg
      libopus
      python3Packages.yt-dlp
      
      # Debug tools
      strace
      gdb
      htop
      netcat
      tcpdump
      lsof
      procps
      
      # Development tools
      git
      vim
    ];

    config = {
      Cmd = [ "${lavalink-rust}/bin/lavalink-rust" ];
      ExposedPorts = {
        "2333/tcp" = {};
        "9090/tcp" = {};
      };
      Env = [
        "PATH=/bin:${pkgs.coreutils}/bin:${pkgs.curl}/bin:${pkgs.ffmpeg}/bin"
        "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        "RUST_LOG=debug"
        "RUST_BACKTRACE=full"
      ];
      WorkingDir = "/app";
    };
  };
}
