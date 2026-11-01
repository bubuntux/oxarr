{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  cachix.enable = false;

  languages.rust = {
    enable = true;
    channel = "stable";
    targets = [ "wasm32-unknown-unknown" ];
  };

  packages = [
    pkgs.cargo-nextest
    pkgs.cargo-watch
    pkgs.gh
    pkgs.sqlite
    pkgs.sqlx-cli
  ];

  enterShell = ''
    echo "🦀 oxarr devenv ready"
    echo "  rust: $(rustc --version 2>/dev/null || echo 'loading...')"
    echo "  cargo: $(cargo --version 2>/dev/null || echo 'loading...')"
  '';
}
