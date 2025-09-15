{
  description = "CNCF-based AutoAgents Infrastructure Stack";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";

    # CNCF components
    linkerd2-proxy = {
      url = "github:linkerd/linkerd2-proxy/main";
      flake = false;
    };
    spin = {
      url = "github:fermyon/spin/v2.4.3";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, linkerd2-proxy, spin }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Build Linkerd2-proxy
        linkerd2-proxy-bin = pkgs.rustPlatform.buildRustPackage {
          pname = "linkerd2-proxy";
          version = "edge";
          src = linkerd2-proxy;

          cargoLock = {
            lockFile = "${linkerd2-proxy}/Cargo.lock";
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            protobuf
            cmake
          ];

          buildInputs = with pkgs; [
            openssl
            systemd
          ];

          meta = with pkgs.lib; {
            description = "Linkerd2-proxy service mesh data plane";
            homepage = "https://linkerd.io";
            license = licenses.asl20;
          };
        };

        # Fermyon Spin
        spin-bin = pkgs.stdenv.mkDerivation {
          pname = "spin";
          version = "2.4.3";
          src = spin;

          nativeBuildInputs = with pkgs; [
            rustToolchain
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
            zlib
          ];

          buildPhase = ''
            cargo build --release
          '';

          installPhase = ''
            mkdir -p $out/bin
            cp target/release/spin $out/bin/
          '';

          meta = with pkgs.lib; {
            description = "Fermyon Spin - Serverless WebAssembly";
            homepage = "https://spin.fermyon.dev";
            license = licenses.asl20;
          };
        };

      in
      {
        packages = {
          linkerd2-proxy = linkerd2-proxy-bin;
          spin = spin-bin;
          default = self.packages.${system}.linkerd2-proxy;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            cargo
            rustc

            # CNCF components
            self.packages.${system}.linkerd2-proxy
            self.packages.${system}.spin

            # Infrastructure tools
            redis
            docker
            docker-compose
            kubectl
            k3s
            nixos-rebuild

            # Development tools
            git
            protobuf
            cmake
            pkg-config
            openssl
            systemd

            # WASM tools
            wasm-pack
            wasmtime
            binaryen

            # Monitoring and observability
            prometheus
            grafana
            jaeger

            # Security tools
            vault
            sops
          ];

          shellHook = ''
            echo "🚀 CNCF AutoAgents Infrastructure Development Environment"
            echo "Available tools:"
            echo "  - linkerd2-proxy: ${self.packages.${system}.linkerd2-proxy}/bin/linkerd2-proxy"
            echo "  - spin: ${self.packages.${system}.spin}/bin/spin"
            echo "  - redis: $(which redis-server)"
            echo "  - docker: $(which docker)"
            echo ""
            export RUST_BACKTRACE=1
            export LINKERD2_PROXY_LOG=info
            export SPIN_LOG=info
          '';
        };

        # NixOS configuration for deployment
        nixosConfigurations.autoagents-infra = nixpkgs.lib.nixosSystem {
          inherit system;
          modules = [
            ./nixos/configuration.nix
          ];
        };
      });
}
