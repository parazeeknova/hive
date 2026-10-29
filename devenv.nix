{ pkgs, ... }:
{
  packages = with pkgs; [
    bun
    nodejs_24
    cargo-deny
    cargo-machete
    cargo-watch
    docker-client
    docker-compose
    pkg-config
    cmake
    clang
    nixfmt
  ];

  languages.rust = {
    enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };

  enterShell = ''
    if [ -z "''${DOCKER_HOST:-}" ] && [ ! -S /var/run/docker.sock ]; then
      for hive_socket in /run/podman/podman.sock "''${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/podman/podman.sock"; do
        if [ -S "$hive_socket" ] && [ -r "$hive_socket" ] && [ -w "$hive_socket" ]; then
          export DOCKER_HOST="unix://$hive_socket"
          break
        fi
      done
      unset hive_socket
    fi
  '';

  env = {
    NEXT_TELEMETRY_DISABLED = "1";
    NX_DAEMON = "false";
  };
}
