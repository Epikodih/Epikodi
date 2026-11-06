# Architecture du monorepo EpiKodi

## Vue d'ensemble

```
epikodi/
├── crates/        → Rust : le cœur (source de vérité, seul à ouvrir la base SQLite)
├── apps/          → Flutter : tv (desktop) + remote (Android)
├── packages/      → Dart partagé par tv ET remote
├── extensions/    → SDK + extensions WASM livrées
├── themes/        → thèmes complets interchangeables
├── fixtures/      → exemples JSON du protocole (le contrat Rust ↔ Dart)
├── scripts/       → setup dev, installation de la démo
├── docs/          → documentation (dont le modèle de données)
└── .github/       → CI/CD
```

**Règle d'or :** les apps Flutter ne touchent jamais la base. Seul le cœur Rust ouvre `epikodi.db` ; les apps lui parlent via le protocole (HTTP + WebSocket, port `7878`).

## Arborescence

```
epikodi/
├── Cargo.toml                 # workspace Rust (members = ["crates/*"])
├── pubspec.yaml               # workspace Dart (apps/* + packages/*)
├── justfile                   # core, tv, remote, setup, lint, test, demo
├── README.md  ARCHITECTURE.md  CHANGELOG.md  LICENSE
├── .env.example  .gitignore  .pre-commit-config.yaml
├── rust-toolchain.toml  rustfmt.toml  clippy.toml  analysis_options.yaml
│
├── crates/                    # ══════ CÔTÉ RUST ══════
│   ├── domain/                # entités + règles pures, AUCUN I/O, AUCUN SQL
│   │   └── src/
│   │       ├── media/         # film, série, saison, épisode, artiste, album,
│   │       │                  #   morceau, podcast + règles de hiérarchie
│   │       ├── profile/       # profils, préférences
│   │       ├── parental/      # règle de visibilité, PIN
│   │       ├── playback/      # reprise, vu / non vu
│   │       ├── queue/  playlist/  source/  device/  app_state/
│   │       ├── extension/     # manifeste, permissions
│   │       ├── ports/         # traits de dépôt (implémentés par storage)
│   │       ├── protocol/      # types des messages cœur ↔ apps
│   │       └── error.rs
│   │
│   ├── storage/               # SQLite, migrations, caches, secrets
│   │   ├── migrations/        # 0001_init.sql (= schéma v3), puis 0002_…
│   │   ├── src/
│   │   │   ├── db.rs          # ouverture, PRAGMA, 1 écrivain + lecteurs
│   │   │   ├── migrate.rs     # applique les migrations (PRAGMA user_version)
│   │   │   ├── json.rs        # colonnes JSON ↔ structs typées
│   │   │   ├── repositories/  # un fichier par table
│   │   │   ├── cache/         # images + réponses d'API (fichiers, pas SQLite)
│   │   │   └── secrets.rs     # identifiants chiffrés
│   │   └── tests/             # SQLite en mémoire, vraies migrations
│   │
│   ├── library/               # scan, reconnaissance, enrichissement
│   │   ├── src/
│   │   │   ├── scanner/       # complet + ciblé (par dossier)
│   │   │   ├── recognition/   # S01E02, 1x02, film + année, tags audio
│   │   │   ├── enrichment.rs
│   │   │   └── manual_fix.rs  # correction manuelle
│   │   └── tests/             # jeu de noms de fichiers difficiles
│   │
│   ├── sources/               # accès fichiers
│   │   └── src/               # source.rs (trait), local.rs, smb.rs (ou webdav/dlna),
│   │                          #   discovery.rs, degraded.rs
│   │
│   ├── metadata/              # API publiques
│   │   └── src/               # provider.rs (trait), tmdb.rs, musicbrainz.rs, mock.rs
│   │
│   ├── extensions/            # hôte WASM
│   │   └── src/               # manifest.rs, permissions.rs, runtime.rs (wasmtime),
│   │                          #   lifecycle.rs, supervisor.rs
│   │
│   ├── api/                   # axum
│   │   └── src/               # http/ (santé, appairage), ws/ (commandes + événements),
│   │                          #   auth/ (appareils appairés), state.rs
│   │
│   └── core-bin/              # binaire epikodi-core : assemble tout
│       └── src/               # main.rs, config.rs, wiring.rs, mdns.rs
│
├── apps/                      # ══════ APPS FLUTTER ══════
│   ├── tv/                    # epikodi_tv (Linux + Windows)
│   │   ├── lib/
│   │   │   ├── core_process/  # lance le cœur en sidecar
│   │   │   ├── navigation/    # flèches / Entrée / Retour
│   │   │   ├── features/      # home, library, player (media_kit), profiles,
│   │   │   │                  #   settings, extensions
│   │   │   └── state/         # état de lecture publié vers le cœur
│   │   ├── test/  linux/  windows/  assets/
│   │   └── pubspec.yaml
│   │
│   └── remote/                # epikodi_remote (Android)
│       ├── lib/
│       │   ├── discovery/     # mDNS + saisie manuelle de l'IP (secours)
│       │   ├── pairing/       # code ou QR
│       │   └── features/      # dpad, playback, browse, queue, send_to_tv
│       ├── test/  android/  assets/
│       └── pubspec.yaml
│
├── packages/                  # ══════ DART PARTAGÉ ══════
│   ├── client/                # epikodi_client : WebSocket/HTTP, modèles, flux d'état,
│   │   │                      #   appairage
│   │   └── test/contract_test.dart   # charge toutes les fixtures
│   └── design_system/         # epikodi_design_system : composants, focus unique,
│                              #   thèmes à chaud, contrastes AA
│
├── extensions/
│   └── sdk/                   # SDK + les 3 extensions livrées
│                              #   (source de contenus, fournisseur d'infos, widget/service)
├── themes/                    # default/  alternative/
├── fixtures/                  # commands/  events/  pairing/   (le contrat)
├── scripts/                   # setup-dev.sh / .ps1, install-demo/
├── docs/
│   ├── adr/                   # stack, qui détient l'état, WASM, protocole, SQLite
│   ├── dev/                   # protocol.md, data-model.md, schema.sql, schema_diagramme.pdf
│   ├── user/  extensions/  accessibility/  brand/
├── media-demo/                # ignoré par git
└── .github/                   # workflows/ (ci.yml, release.yml), templates, CODEOWNERS
```

---

## Le cœur Rust et SQLite

### Qui fait quoi

| Crate | Rôle | Touche SQLite ? |
|---|---|---|
| `domain` | Entités, règles métier, **traits** de dépôt (`ports/`) | Non |
| `storage` | Base, migrations, dépôts, caches, secrets | **Oui, seule crate** |
| `library` | Scan, reconnaissance, enrichissement | Non (via les ports) |
| `sources` | Lecture des fichiers (local, réseau) | Non |
| `metadata` | Clients TMDB / MusicBrainz | Non |
| `extensions` | Runtime WASM, permissions, supervision | Non (via les ports) |
| `api` | HTTP / WebSocket pour les apps | Via `storage` |
| `core-bin` | Câblage : ouvre la base, injecte les dépôts | L'ouvre, la confie à `storage` |

### Règle de dépendance (celle du README)

```
domain  ◄──  storage, library, sources, metadata, extensions  ◄──  api  ◄──  core-bin
```

- `domain` ne dépend de **rien**.
- `api` dépend de `domain`, `storage`, `library`, `extensions` ; `core-bin` de toutes.
- `library`, `sources`, `metadata`, `extensions` ne dépendent que de `domain`. Pour lire ou écrire en base (ou appeler un fournisseur), elles utilisent les **traits de `domain/ports/`** ; `core-bin` leur injecte les vraies implémentations. On les teste donc avec de faux dépôts, sans base.
- *À trancher en équipe :* le README place `MetadataProvider` dans `metadata`. Si on le garde là, la dépendance `library → metadata` est à noter dans un ADR (règle du README). Sinon, on déplace le trait dans `domain/ports/`.

### La base (`storage`)

- **Fichier** `epikodi.db` dans le dossier de données (réglé dans `core-bin/config.rs`).
- **Pilote** suggéré : `rusqlite` (SQLite embarqué, donc fonctions JSON toujours dispos) + `spawn_blocking` depuis le code async. Le reste du workspace ne le voit pas, on peut changer.
- **Connexions** : un écrivain + quelques lecteurs, mode WAL.
- **PRAGMA à chaque connexion** : `foreign_keys = ON` (indispensable, le schéma repose sur `ON DELETE CASCADE / SET NULL`), `journal_mode = WAL`, `busy_timeout`.
- **Migrations** : `0001_init.sql` = schéma v3 actuel, suivi par `PRAGMA user_version`. Ne jamais modifier une migration déjà livrée : on ajoute `0002_…`. Un test vérifie qu'une base neuve correspond à `docs/dev/schema.sql`.

### Table → code

| Tables SQL | Entité (`domain`) | Écrit par | Lu par |
|---|---|---|---|
| `user`, `device` | profil, appareil | api | api |
| `source` | source | api, sources (statut) | library, sources |
| `media`, `media_details` | média, fiche détaillée | **library** (scan, enrichissement) | api |
| `watch_progress`, `favorite` | progression, favori | api (événements de lecture) | api |
| `playlist`, `playlist_item` | liste de lecture | api | api |
| `queue_item`, `app_state` | file d'attente, état global | api, core-bin (création au démarrage) | api |
| `extension` | extension | extensions (cycle de vie, supervision) | api |

### Colonnes JSON : toujours typées

`user.preferences`, `media_details.details`, `extension.permissions` sont exposées au reste du code comme des **structs typées** (jamais du texte brut), avec valeurs par défaut si une clé manque. `extension.settings` reste du JSON libre (propre à chaque extension). Le `CHECK (json_valid(...))` du SQL n'est que le filet de sécurité.

### Règles que le SQL ne vérifie pas (dans `domain`, testées)

- Hiérarchie des `kind` : `season` sous une `series`, `episode` sous une `season`, `album` sous un `artist`, `track` sous un `album`, `podcast_episode` sous un `podcast`.
- Profil restreint : ne voit que les médias dont `age_rating` est connu et `<= age`. Note inconnue (`NULL`) = masqué.
- `match_status = 'manual'` n'est jamais écrasé par un enrichissement automatique.
- La progression est stockée sur la feuille (film, épisode, morceau) ; le « vu » d'une série ou d'une saison se **déduit** de ses épisodes.
- `app_state` n'a qu'une ligne (`id = 1`), créée une fois au démarrage.

### Filtre parental

Il se fait **dans la requête SQL**, pas après coup (sinon pagination et compteurs sont faux). Toute méthode de `MediaRepo` qui liste du catalogue prend un argument `Visibility` obligatoire, pour qu'on ne puisse pas l'oublier.

### Le scan et la base

- **Identité d'un fichier = `media.uri`** (unique) : un re-scan met à jour, ne duplique pas.
- **Fichier disparu = `status = 'missing'`**, la ligne est **gardée** : favoris, playlists et progression survivent à un NAS débranché.
- **Fichier déplacé** : retrouvé via `fingerprint`, on met à jour l'`uri` de l'ancienne ligne.
- **Scan ciblé** d'un dossier grâce à `parent_dir` (indexé).
- Séries, saisons, artistes, albums sont créés à la demande puis réutilisés.
- Une transaction par fichier (ou petit lot) : un crash en plein scan ne laisse pas une série à moitié écrite.

### La lecture et la base

L'app TV publie sa position, le cœur écrit dans `watch_progress` (avec limitation de fréquence, et toujours à la pause / l'arrêt). La télécommande envoie ses commandes au cœur, qui met à jour `queue_item` et `app_state`.

### Données sensibles

| Colonne | Protection |
|---|---|
| `app_state.parental_pin_hash` | hash (argon2 ou équivalent), jamais le PIN |
| `device.token_hash` | seul le hash du jeton est stocké |
| `source.credentials_encrypted` | blob chiffré ; la clé n'est **pas** dans la base |

Les identifiants n'apparaissent ni dans les logs, ni dans les messages du protocole, ni dans `fixtures/`.

### Tests Rust

| Niveau | Quoi |
|---|---|
| `domain` | hiérarchie, règle parentale, reprise / vu, valeurs par défaut JSON |
| `storage` | SQLite **en mémoire** avec les vraies migrations : cascades, idempotence du scan, filtre parental |
| `library` | noms de fichiers difficiles, avec un faux dépôt |
| `api` | commandes / événements contre `fixtures/*.json` |

---

## Dart (apps ↔ packages partagés)

```
apps/tv      ──┐
               ├──►  packages/client          (protocole, connexion au cœur)
apps/remote  ──┤
               └──►  packages/design_system   (composants, focus, thèmes)

packages/design_system  ──►  (ne dépend pas de client)
packages/client         ──►  (ne dépend pas de design_system)
```

- `tv` et `remote` ne se connaissent pas : ils partagent **uniquement** `packages/`.
- La télécommande parle uniquement au cœur, jamais directement à l'app TV.

## Contrat Rust ↔ Dart

```
docs/dev/protocol.md  →  fixtures/*.json
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
   Test Rust (crates/api)       Test Dart (packages/client)
```

Si le protocole change, les tests cassent des deux côtés. Le schéma SQL ne fait **pas** partie du contrat : c'est un détail interne du cœur, il peut évoluer par migration tant que les messages du protocole ne changent pas.

## Récapitulatif

| Zone | Dossier | Package | Langage |
|---|---|---|---|
| Cœur | `crates/domain` | `epikodi-domain` | Rust |
| Cœur | `crates/storage` | `epikodi-storage` | Rust |
| Cœur | `crates/library` | `epikodi-library` | Rust |
| Cœur | `crates/sources` | `epikodi-sources` | Rust |
| Cœur | `crates/metadata` | `epikodi-metadata` | Rust |
| Cœur | `crates/extensions` | `epikodi-extensions` | Rust |
| Cœur | `crates/api` | `epikodi-api` | Rust |
| Cœur | `crates/core-bin` | `epikodi-core` | Rust |
| App | `apps/tv` | `epikodi_tv` | Dart / Flutter |
| App | `apps/remote` | `epikodi_remote` | Dart / Flutter |
| Partagé | `packages/client` | `epikodi_client` | Dart |
| Partagé | `packages/design_system` | `epikodi_design_system` | Dart |

## Notes

- Le `pubspec.yaml` racine active un *workspace Dart* (Dart ≥ 3.6) : un seul `flutter pub get` couvre tout le monorepo. Sinon, utiliser `melos`.
- Si une table change, mettre à jour dans cet ordre : migration, entité `domain`, port, dépôt, `data-model.md`, diagramme.
- Le nom commercial du produit reste centralisé dans **un seul** fichier de configuration (voir `docs/brand/`).
