# Backend Rust : lancer, tester, essayer l'API

## Prérequis

- Rust stable (`rustup`, voir `rust-toolchain.toml`) et un compilateur C (SQLite est embarqué).
- Facultatif : `just`, `curl`, `python3` (pour lire le JSON dans les exemples).

## Lancer le serveur

```bash
just core            # ou : cargo run -p epikodi-core
```

Il lit `epikodi.toml` (dossier courant), écoute sur `0.0.0.0:7878`, crée
`data/epikodi.db` (et `data/secret.key`, la clé qui chiffre les identifiants
des partages) puis s'annonce en mDNS. Logs plus bavards :
`RUST_LOG=debug just core`. Clé TMDB facultative : voir `.env.example`.

Ordre de démarrage : le port est ouvert tout de suite (la TV reçoit `503`
tant que la base n'est pas prête), puis la base est ouverte ; si elle échoue,
le serveur s'arrête avec un message clair. Port déjà pris : erreur immédiate.

## Lancer les tests

```bash
just test-rust                     # tout le backend (comme la CI)
cargo test -p epikodi-api          # seulement l'API
cargo test -p epikodi-library cas_difficiles   # un seul test
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check
```

| Crate | Ce qui est testé |
|---|---|
| `domain` | hiérarchie des médias, règle parentale, reprise / vu, file, permissions, manifeste |
| `storage` | SQLite en mémoire avec les vraies migrations : cascades, filtre parental, chiffrement, unicité |
| `library` | noms de fichiers difficiles, scan incrémental (ajout, modif, déplacement, disparition, source injoignable), enrichissement hors ligne |
| `sources` | listage local, scan ciblé, empreinte, délai dépassé, SMB |
| `metadata` | TMDB / MusicBrainz avec réponses enregistrées (aucun réseau) |
| `extensions` | installation dossier / zip (zip-slip refusé), mise à jour, réglages, superviseur (plantage, blocage) |
| `api` | contrat `fixtures/`, appairage, comptes / PIN, catalogue, lecteur, file, SSE, sources + scan réel, extensions |

## Essayer l'API à la main

Dans un second terminal, serveur lancé. Les appels « TV » viennent de
`127.0.0.1` sans jeton ; pour simuler le téléphone, on utilise l'IP du réseau
local (sinon le serveur vous prend pour la TV).

```bash
B=http://127.0.0.1:7878
LAN=http://$(hostname -I | awk '{print $1}'):7878
curl -s $B/api/health
```

1. La TV se connecte et affiche un code :

```bash
CODE=$(curl -s -X POST $B/api/tv/connect -H 'Content-Type: application/json' -d '{"model":"Desktop","platform":"linux","app_version":"0.1.0"}' | python3 -c 'import sys,json;print(json.load(sys.stdin)["pairing"]["code"])'); echo $CODE
```

2. Le téléphone s'appaire et reçoit son jeton :

```bash
TOKEN=$(curl -s -X POST $LAN/api/remote/pair -H 'Content-Type: application/json' -d "{\"code\":\"$CODE\",\"device\":{\"name\":\"Pixel 7\",\"install_id\":\"test\",\"platform\":\"android\"}}" | python3 -c 'import sys,json;print(json.load(sys.stdin)["token"])'); echo $TOKEN
```

3. Un compte adulte (il devient actif), puis une source et le scan :

```bash
curl -s -X POST $LAN/api/profiles -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{"name":"Marc","age":35,"is_restricted":false,"pin":"1234"}'
```

```bash
curl -s -X POST $LAN/api/sources -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d "{\"name\":\"Démo\",\"protocol\":\"local\",\"uri\":\"file://$HOME/Vidéos\"}"
```

```bash
curl -s $LAN/api/library/scan -H "Authorization: Bearer $TOKEN"
```

4. Parcourir et lire :

```bash
curl -s "$LAN/api/media?limit=10" -H "Authorization: Bearer $TOKEN"
```

```bash
curl -s -X POST $LAN/api/player/play -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{"media_id":1}'
```

```bash
curl -s -X POST $LAN/api/player/command -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' -d '{"action":"volume","level":40}'
```

5. Le temps réel (avec [websocat](https://github.com/vi/websocat), facultatif) :

```bash
websocat "ws://127.0.0.1:7878/api/ws?token=$TOKEN"
```

Et le flux de la TV : `curl -N "$B/api/tv/events?tv_session_id=<id>"`.

Toutes les routes : `docs/dev/protocol.md`.

## Avec l'app TV et la télécommande

```bash
just core
```

```bash
just tv linux        # ou windows / macos
```

La TV se connecte à `localhost:7878` et affiche le code et le QR. Sur le
téléphone, le port 7878 doit être joignable (même réseau Wi-Fi, pare-feu ouvert).

**Sous WSL2** : le téléphone ne voit pas l'IP interne de WSL. Soit activer le
réseau « miroir » (`networkingMode=mirrored` dans `%UserProfile%\.wslconfig`,
puis `wsl --shutdown`), soit rediriger le port depuis un PowerShell
administrateur :

```powershell
netsh interface portproxy add v4tov4 listenport=7878 listenaddress=0.0.0.0 connectport=7878 connectaddress=<IP de WSL>
```

et mettre l'IP Windows du PC dans `epikodi.toml` (`host = "192.168.x.x"`) pour
que le QR code pointe au bon endroit.

## Limites connues

- Extensions : le format, les permissions, l'installation / mise à jour / désinstallation à chaud et le superviseur (plantage, blocage) sont faits. Le moteur d'exécution WASM (wasmtime) qui implémente `ExtensionRuntime` reste à brancher.
- Sources réseau : SMB passe par le système (chemin UNC sous Windows, partage monté par GVFS ou `SMB_MOUNT_ROOT` sous Linux). DLNA, NFS et WebDAV renvoient « pas encore pris en charge ».
- Podcasts (flux RSS) : le modèle de données est prêt, l'abonnement n'est pas encore implémenté.
