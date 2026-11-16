# Sous Windows, just n'a pas de `sh` : on passe par cmd (gère `cd x && y`).
set windows-shell := ["cmd.exe", "/c"]

core:
    cargo run -p epikodi-core
# Lance l'app TV. Windows par défaut ; sinon : `just tv macos` ou `just tv linux`.
tv device="windows":
    cd apps/tv && flutter run -d {{device}}
remote:
    cd apps/remote && flutter run
setup:
    ./scripts/setup-dev.sh
lint:
    cargo fmt --check && cargo clippy --workspace -- -D warnings && flutter analyze packages/client packages/design_system apps/tv apps/remote

# Marche sous Windows, macOS et Linux (script Dart, pas de bash requis).
# Tous les tests (Rust + Flutter), en une commande, sans connexion réseau.
test:
    dart scripts/test.dart

# Seulement les crates Rust (cœur, API, extensions...).
test-rust:
    dart scripts/test.dart rust

# Seulement les apps et packages Flutter.
test-flutter:
    dart scripts/test.dart flutter

demo:
    ./scripts/install-demo/install.sh
