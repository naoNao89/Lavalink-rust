# Deployment configurations for different environments
{ pkgs, lib, ... }:

{
  # Development deployment
  development = {
    hostname = "localhost";
    profiles.system = {
      user = "root";
      path = pkgs.deploy-rs.lib.x86_64-linux.activate.nixos {
        nodes.development = {
          hostname = "localhost";
          profiles.system = {
            user = "root";
            path = ./configurations/development.nix;
          };
        };
      };
    };
  };

  # Staging deployment
  staging = {
    hostname = "staging.example.com";
    profiles.system = {
      user = "root";
      path = pkgs.deploy-rs.lib.x86_64-linux.activate.nixos {
        nodes.staging = {
          hostname = "staging.example.com";
          profiles.system = {
            user = "root";
            path = ./configurations/staging.nix;
          };
        };
      };
    };
  };

  # Production deployment
  production = {
    hostname = "lavalink.example.com";
    profiles.system = {
      user = "root";
      path = pkgs.deploy-rs.lib.x86_64-linux.activate.nixos {
        nodes.production = {
          hostname = "lavalink.example.com";
          profiles.system = {
            user = "root";
            path = ./configurations/production.nix;
          };
        };
      };
    };
  };
}
