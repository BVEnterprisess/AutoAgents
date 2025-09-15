{ config, pkgs, ... }:

{
  # System configuration
  system.stateVersion = "23.11";

  # Boot configuration
  boot = {
    loader = {
      systemd-boot.enable = true;
      efi.canTouchEfiVariables = true;
    };
    kernelModules = [ "br_netfilter" ];
    kernel.sysctl = {
      "net.bridge.bridge-nf-call-iptables" = 1;
      "net.ipv4.ip_forward" = 1;
      "net.bridge.bridge-nf-call-ip6tables" = 1;
      "net.ipv6.conf.all.forwarding" = 1;
    };
  };

  # Networking
  networking = {
    hostName = "autoagents-infra";
    useDHCP = false;
    interfaces.ens33.useDHCP = true;
    firewall = {
      enable = true;
      allowedTCPPorts = [
        22    # SSH
        80    # HTTP
        443   # HTTPS
        6379  # Redis
        8080  # Linkerd admin
        8081  # Curation Engine
        8082  # WASM services
        9090  # Prometheus
        3000  # Grafana
        16686 # Jaeger
      ];
      allowedUDPPorts = [
        53    # DNS
      ];
    };
  };

  # Security
  security = {
    sudo.wheelNeedsPassword = false;
    apparmor.enable = true;
  };

  # Users
  users.users.autoagents = {
    isNormalUser = true;
    extraGroups = [ "wheel" "docker" "networkmanager" ];
    openssh.authorizedKeys.keys = [
      # Add your SSH public key here
    ];
  };

  # Services
  services = {
    # SSH
    openssh = {
      enable = true;
      settings = {
        PermitRootLogin = "no";
        PasswordAuthentication = false;
      };
    };

    # Redis
    redis = {
      enable = true;
      bind = "127.0.0.1";
      port = 6379;
      settings = {
        maxmemory = "512mb";
        maxmemory-policy = "allkeys-lru";
        tcp-keepalive = 300;
        timeout = 300;
      };
    };

    # Docker
    docker = {
      enable = true;
      liveRestore = false;
    };

    # Prometheus monitoring
    prometheus = {
      enable = true;
      port = 9090;
      exporters = {
        node = {
          enable = true;
          enabledCollectors = [ "systemd" ];
          port = 9100;
        };
        redis = {
          enable = true;
          port = 9121;
        };
      };
      scrapeConfigs = [
        {
          job_name = "autoagents";
          static_configs = [{
            targets = [
              "localhost:8081"  # Curation Engine
              "localhost:8082"  # WASM services
              "localhost:9100"  # Node exporter
              "localhost:9121"  # Redis exporter
            ];
          }];
        }
      ];
    };

    # Grafana
    grafana = {
      enable = true;
      port = 3000;
      domain = "localhost";
      security = {
        adminUser = "admin";
        adminPassword = "autoagents";
      };
    };

    # Jaeger tracing
    jaeger = {
      enable = true;
      extraConfig = {
        SPAN_STORAGE_TYPE = "memory";
        MEMORY_MAX_TRACES = "100000";
      };
    };
  };

  # Environment
  environment = {
    systemPackages = with pkgs; [
      # Core tools
      git
      vim
      htop
      curl
      wget
      jq
      yq

      # CNCF tools
      kubectl
      k3s
      linkerd
      istioctl

      # Development
      rustc
      cargo
      rust-analyzer
      wasm-pack
      wasmtime

      # Infrastructure
      docker-compose
      terraform
      ansible

      # Monitoring
      prometheus
      grafana
      jaeger

      # Security
      vault
      sops
      age
    ];

    variables = {
      RUST_BACKTRACE = "1";
      LINKERD2_PROXY_LOG = "info";
      SPIN_LOG = "info";
    };
  };

  # Virtualisation
  virtualisation = {
    docker.enable = true;
    podman.enable = true;
  };

  # Nix settings
  nix = {
    settings = {
      experimental-features = [ "nix-command" "flakes" ];
      auto-optimise-store = true;
    };
    gc = {
      automatic = true;
      dates = "weekly";
      options = "--delete-older-than 30d";
    };
  };

  # Systemd services for AutoAgents components
  systemd.services = {
    # Linkerd2-proxy service
    linkerd2-proxy = {
      description = "Linkerd2 Proxy Service Mesh Data Plane";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.linkerd2-proxy}/bin/linkerd2-proxy";
        Restart = "always";
        User = "autoagents";
        Environment = [
          "LINKERD2_PROXY_LOG=info"
          "LINKERD2_PROXY_CONTROL_URL=http://localhost:4191"
        ];
      };
    };

    # Curation Engine service
    curation-engine = {
      description = "AutoAgents Curation Engine Control Plane";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" "redis.service" ];
      serviceConfig = {
        ExecStart = "/var/lib/autoagents/curation-engine/bin/curation-engine";
        Restart = "always";
        User = "autoagents";
        WorkingDirectory = "/var/lib/autoagents";
        Environment = [
          "REDIS_URL=redis://localhost:6379"
          "RUST_LOG=info"
        ];
      };
    };

    # WASM Runtime service
    wasm-runtime = {
      description = "Fermyon Spin WASM Runtime";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.spin}/bin/spin up --listen 0.0.0.0:8082";
        Restart = "always";
        User = "autoagents";
        WorkingDirectory = "/var/lib/autoagents/wasm";
        Environment = [
          "SPIN_LOG=info"
          "SPIN_WORKING_DIR=/var/lib/autoagents/wasm"
        ];
      };
    };
  };

  # File systems
  fileSystems."/var/lib/autoagents" = {
    device = "/dev/disk/by-label/autoagents-data";
    fsType = "ext4";
    options = [ "defaults" "noatime" ];
  };
}
