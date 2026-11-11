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
    pkgs.ffmpeg-headless
    pkgs.gh
    pkgs.sea-orm-cli
    pkgs.sqlite
  ];

  # Local development database, shared by the db-* scripts and `cargo run -p oxarr-server`.
  env.DATABASE_URL = "sqlite://./oxarr.db?mode=rwc";
  env.RUST_LOG = "oxarr=debug,tower_http=debug,info";
  env.PORT = "8080";

  processes.server.exec = "cargo-watch -x 'run -p oxarr-server'";

  scripts = {
    server-run = {
      description = "Start the oxarr server daemon (auto-connects SQLite and runs migrations)";
      exec = ''cargo run -p oxarr-server "$@"'';
    };
    server-watch = {
      description = "Start the oxarr server in watch mode with live reload on changes";
      exec = ''cargo-watch -x "run -p oxarr-server"'';
    };
    db-up = {
      description = "Apply pending migrations (extra args: -n <count>)";
      exec = ''cargo run -q -p oxarr-migration --features cli -- up "$@"'';
    };
    db-down = {
      description = "Revert the last migration (extra args: -n <count>)";
      exec = ''cargo run -q -p oxarr-migration --features cli -- down "$@"'';
    };
    db-status = {
      description = "Show applied and pending migrations";
      exec = ''cargo run -q -p oxarr-migration --features cli -- status "$@"'';
    };
    db-fresh = {
      description = "Drop every table and re-apply all migrations";
      exec = ''cargo run -q -p oxarr-migration --features cli -- fresh "$@"'';
    };
    db-new = {
      description = "Scaffold a migration: db-new <snake_case_name>";
      exec = ''sea-orm-cli migrate generate -d oxarr/migration "$@"'';
    };
    db-entities = {
      description = "Generate dense entities from the migrated DB into a scratch dir to diff against oxarr/core/src/db/models";
      exec = ''
        db-fresh
        sea-orm-cli generate entity \
          --entity-format dense \
          --output-dir "''${1:-target/generated-entities}"
      '';
    };
  };

  enterShell = ''
    echo "🦀 oxarr devenv ready"
    echo "  rust:   $(rustc --version 2>/dev/null || echo 'loading...')"
    echo "  cargo:  $(cargo --version 2>/dev/null || echo 'loading...')"
    echo "  server: server-run | server-watch | devenv up"
    echo "  url:    http://localhost:''${PORT:-8080}"
    echo "  db:     db-up | db-down | db-status | db-fresh | db-new <name> | db-entities"
  '';
}
