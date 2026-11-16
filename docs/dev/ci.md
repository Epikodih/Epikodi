# CI/CD et déroulé d'une contribution

## Le parcours d'une modification

1. Tu crées une branche depuis `dev` (`feature/...`, `fix/...`) et tu pushes.
2. **`auto-pr.yml`** ouvre automatiquement une pull request vers `dev`
   (une seule par branche), avec le modèle `.github/pull_request_template.md`.
   Les code owners (`.github/CODEOWNERS`) sont demandés en relecteurs,
   sauf l'auteur du push.
3. **`ci.yml`** tourne sur le même commit. Ses résultats s'affichent sur la PR :
   - `rust` : `cargo fmt --check`, `cargo clippy -D warnings`, tests Rust
   - `flutter` : `flutter analyze` puis tests de la TV, de la télécommande et des packages
   - `build (linux|windows|macos)` : build du cœur et de l'app TV
4. Chaque nouveau push sur la branche relance la CI. La PR existante se met à jour.
5. Quand la CI est verte **et** qu'un code owner a approuvé, tu merges dans `dev`.
   Le push sur `dev` relance la CI.
6. **`release.yml`** : un tag `v*` construit les paquets (Linux, Windows, macOS,
   APK Android) et publie une release GitHub.

## Lancer les tests en local

Même commande que la CI, sans connexion réseau. Marche sous Windows
(PowerShell, cmd ou Git Bash), macOS et Linux : `just` lance
`scripts/test.dart`, un script Dart (installé avec Flutter), sans bash requis.

```bash
just test            # tout
just test-rust       # crates Rust seulement
just test-flutter    # apps et packages Flutter seulement
```

Lancer un seul fichier ou un seul test :

```bash
cd apps/tv && flutter test test/pairing_screen_test.dart
cd apps/tv && flutter test --plain-name "Nouveau code"
cargo test -p epikodi-library nom_du_test
```

## Réglages GitHub à faire une fois (admin du dépôt)

Ces réglages ne se mettent pas dans un fichier. Sans eux, les workflows
tournent, mais rien n'empêche de merger une PR rouge.

1. **Settings → Actions → General → Workflow permissions** : cocher
   *Allow GitHub Actions to create and approve pull requests*.
   Le dépôt appartient à l'organisation Epikodih : le même réglage doit aussi
   être autorisé dans les paramètres de l'organisation. Sinon `auto-pr` échoue
   avec « GitHub Actions is not permitted to create or approve pull requests ».
2. **Settings → Collaborators and teams** : `malo-coet` et `loicosternaud`
   doivent avoir l'accès *Write*. Sans ce droit, GitHub ignore la ligne
   CODEOWNERS et ne les demande pas en relecteurs.
3. **Settings → Branches → règle pour `dev`, puis la même pour `main`** :
   - *Require a pull request before merging*
     - *Require approvals* : 1
     - *Require review from Code Owners*
   - *Require status checks to pass before merging*, avec les checks
     `rust`, `flutter`, `build (linux)`, `build (windows)`, `build (macos)`
     (ils apparaissent dans la liste après un premier run de la CI)
   - *Do not allow bypassing the above settings*

## Limites connues

- Une PR créée par `auto-pr` a pour auteur `github-actions`. C'est normal :
  les commits restent ceux de leur auteur.
- Si la PR d'une branche a été fermée sans merge, le push suivant en rouvre une nouvelle.
