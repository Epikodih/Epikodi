# API du cœur (HTTP, WebSocket, SSE)

Référence de toutes les routes servies par le cœur Rust (`crates/api`), port
`7878`. Le contrat partagé avec les apps Flutter reste `fixtures/` : chaque
fichier y est vérifié côté Rust (`crates/api/tests/contract.rs`) et côté Dart
(`packages/client/test/contract_test.dart`).

Conventions : JSON en `snake_case`, dates ISO 8601 UTC (`2026-11-07T18:31:12Z`),
erreurs toujours au format `{"error": {"code", "message", "details"}}`
(`details` vaut `null` s'il n'y a rien à préciser). Les apps se basent sur
`code`, jamais sur `message`.

## Qui a le droit de quoi

| Appelant | Comment il est reconnu | Accès |
|---|---|---|
| Tout le monde | — | `GET /api/health`, `POST /api/remote/pair` |
| App TV | boucle locale (127.0.0.1 / ::1) **sans** en-tête `Authorization` | routes `/api/tv/*`, `POST /api/player/state`, et toutes les routes authentifiées |
| Télécommande | `Authorization: Bearer <jeton>` (ou `?token=` sur `/api/ws`) | routes authentifiées |

Un en-tête `Authorization` présent est toujours vérifié, même en local. Jeton
inconnu, révoqué ou expiré (30 jours) : `401 unauthorized`, la télécommande
retourne à l'écran d'appairage.

Le **profil actif** est commun à la TV et aux télécommandes (c'est celui
affiché sur la télévision). Selon la route, il faut en plus :

| Exigence | Erreur si absente |
|---|---|
| un profil choisi (invité ou compte) | `403 profile_required` |
| un compte (pas l'invité) | `403 guest_forbidden` |
| un compte **non restreint** (adulte) | `403 admin_required` |

Le contrôle parental est appliqué par le serveur, dans la requête SQL : un
profil restreint (l'invité a l'âge 0) ne voit que les médias dont la
classification est connue et inférieure ou égale à son âge. Un épisode ou un
morceau sans classification hérite de celle de sa saison / série / album. Un
média masqué répond `404 media_not_found` (on ne révèle pas qu'il existe).

## Démarrage et appairage

| Route | Accès | Réponse |
|---|---|---|
| `GET /api/health` | public | `health/ok.json` ; `503 {"status": "starting"}` tant que la base n'est pas prête |
| `POST /api/tv/connect` | local | `201` `tv/connect_response.json` : session TV, code à 6 chiffres, QR |
| `POST /api/tv/pairing/refresh` | local | nouveau code ; `404 tv_session_not_found` après un redémarrage du serveur |
| `GET /api/tv/status?tv_session_id=` | local | `tv/status.json` : `phase` = `pairing` → `paired` → `ready` |
| `GET /api/tv/events?tv_session_id=` | local | flux SSE : un `tv/status` à l'ouverture puis à chaque changement |
| `POST /api/remote/pair` | public | `pairing/response.json` : jeton (30 jours) |

Règles du code : aléatoire sûr, usage unique, 5 minutes, 5 essais. Erreurs :
`403 pairing_code_invalid` (`details.attempts_left`), `410 pairing_code_expired`,
`429 pairing_too_many_attempts` (`details.retry_after_s`, code renouvelé). Le QR
contient `mediacenter://pair?host=<ip>&port=7878&sid=<session>`, jamais le code.
Un appairage avec le même `install_id` révoque l'ancien jeton.

## Comptes et session

| Route | Accès | Notes |
|---|---|---|
| `GET /api/profiles` | jeton | invité, droit de créer, comptes (`profiles/list.json`) |
| `POST /api/profiles` | jeton, pas l'invité | `201`, le compte devient actif. PIN 4 chiffres obligatoire si non restreint, interdit sinon. Créer un compte non restreint alors qu'un adulte existe : `admin_required` sans adulte connecté |
| `GET /api/profiles/{id}` | jeton | `profiles/get_response.json` |
| `PATCH /api/profiles/{id}` | compte | champs facultatifs. Toucher à `age` / `is_restricted` / `new_pin` : PIN du compte (`pin_required`, `pin_invalid`, `pin_locked`), ou un adulte connecté pour un compte restreint. Un profil restreint ne modifie que lui-même |
| `DELETE /api/profiles/{id}` | compte | corps facultatif `{"pin": "1234"}` |
| `GET /api/session` | jeton | appareil (`null` pour la TV) et profil actif |
| `POST /api/session/profile` | jeton | `{"type": "guest"}` ou `{"type": "account", "profile_id", "pin"?}` |
| `DELETE /api/session/profile` | jeton | retour au choix du profil (`204`) |
| `DELETE /api/session` | jeton | déconnexion : le jeton est révoqué (`204`) |
| `GET /api/me/preferences` | profil | préférences typées (valeurs par défaut pour l'invité) |
| `PATCH /api/me/preferences` | compte | fusion partielle (`{"theme": "alternative", "subtitles": {"size": 1.5}}`) |
| `GET /api/themes` | jeton | thèmes installés (dossier `themes/`) |
| `GET /api/devices` | adulte | appareils appairés (`current` = celui qui appelle) |
| `DELETE /api/devices/{id}` | soi-même ou adulte | révoque un appareil perdu |

PIN : 5 essais puis verrouillage 60 s (`429 pin_locked`, `details.retry_after_s`).

## Médiathèque

| Route | Accès | Notes |
|---|---|---|
| `GET /api/home` | profil | reprise, derniers ajouts, favoris, rubriques (compteurs), widgets d'extensions |
| `GET /api/media` | profil | `kind`, `parent_id`, `limit` (50, max 500), `offset`, `q` (recherche sans accents), `genre`, `year`, `sort` (`title`, `year`, `added`). Erreurs : `422 invalid_query`, `404 media_not_found` |
| `GET /api/media/genres`, `/years` | profil | `{"genres": [{"value", "count"}]}` |
| `GET /api/media/recent`, `/continue` | profil | `?limit=` |
| `GET /api/media/{id}` | profil | `media/detail.json` : fiche, fil d'Ariane, progression, vu, favori ; `playback_uri` seulement pour l'app TV |
| `GET /api/media/{id}/poster` | profil | image du cache local |
| `GET /api/media/{id}/next` | profil | `{"media": ...|null}` (épisode suivant) |
| `PUT /api/media/{id}/progress` | compte | `{"position_s", "duration_s"?}` ; vu automatiquement à 90 % |
| `PUT /api/media/{id}/watched` | compte | `{"watched": true}` ; une série / saison / album s'applique à toutes ses feuilles |
| `PATCH /api/media/{id}` | adulte | correction manuelle (`title`, `year`, `age_rating`, `sort_number`) → `match_status: manual` |
| `GET /api/media/{id}/matches?q=&year=` | adulte | résultats proposés par les fournisseurs |
| `POST /api/media/{id}/match` | adulte | `{"provider", "external_id"}` : applique le résultat choisi |
| `GET /api/favorites`, `PUT`/`DELETE /api/favorites/{id}` | compte | |
| `GET`/`POST /api/playlists`, `GET`/`PATCH`/`DELETE /api/playlists/{id}` | compte | |
| `POST /api/playlists/{id}/items`, `DELETE /api/playlists/{id}/items/{position}` | compte | |

## Sources et scan

| Route | Accès | Notes |
|---|---|---|
| `GET /api/sources`, `GET /api/sources/{id}` | profil | les identifiants ne sont jamais renvoyés (`has_credentials`) |
| `POST /api/sources` | adulte | `{"name", "protocol": "local"|"smb", "uri", "credentials"?}` → `201` et premier scan lancé |
| `PATCH /api/sources/{id}` | adulte | `name`, `credentials` (`null` pour les effacer) |
| `DELETE /api/sources/{id}` | adulte | supprime la source et ses médias |
| `GET /api/sources/discover` | adulte | partages SMB annoncés en mDNS |
| `POST /api/sources/{id}/scan?dir=` | adulte | `202` ; `dir` = scan ciblé d'un dossier |
| `POST /api/library/scan` | adulte | `202`, toutes les sources ; `409 scan_in_progress` |
| `GET /api/library/scan` | profil | bilan du dernier scan (ajoutés, modifiés, déplacés, disparus, enrichis, erreurs, durée) |

Le scan est incrémental : un fichier inchangé n'est pas réécrit, un fichier
disparu reste en base (`status: missing`), un fichier déplacé garde son
identifiant (favoris et progression conservés). Une source injoignable passe
`offline` sans rien marquer disparu.

## Lecture et télécommande

| Route | Accès | Notes |
|---|---|---|
| `GET /api/player` | jeton | `player/state.json` |
| `POST /api/player/play` | profil | un seul de `media_id` (toutes ses feuilles ; une série reprend au premier épisode non vu), `queue_item_id`, `playlist_id` ; `resume` (vrai par défaut) |
| `POST /api/player/command` | jeton | `{"action": ...}` : `play`, `pause`, `toggle`, `stop`, `next`, `previous`, `seek` (`position_s`), `seek_relative` (`delta_s`), `volume` (`level`), `volume_up`, `volume_down`, `mute`, `unmute`, `audio_track` (`id`), `subtitle_track` (`id` ou `null`), `subtitle_delay` (`delay_ms`), `shuffle` (`enabled`), `repeat` (`mode`: `off`/`one`/`all`) |
| `POST /api/player/state` | **local** | l'app TV publie l'état réel (`status`, `position_s`, `duration_s`, pistes) ; le cœur sauvegarde la reprise (toutes les 10 s, et à la pause / l'arrêt / la fin) et enchaîne le suivant à `ended` |
| `POST /api/player/cast` | jeton | « Envoyer vers la télé » : `{"url": "https://...", "kind": "video"|"audio", "title"?}` |
| `POST /api/player/cast/upload?name=x.mp3` | jeton | corps brut : un fichier du téléphone (500 Mo max) |
| `POST /api/remote/input` | jeton | `{"key": "up"|"down"|"left"|"right"|"ok"|"back"|"home"|"menu"|"info"|"play_pause"}` |
| `GET /api/queue` | profil | `queue/list.json` |
| `POST /api/queue` | profil | `{"media_ids": [...], "mode": "append"|"next"|"replace"}` |
| `PATCH /api/queue/{id}` | profil | `{"position": 0}` |
| `DELETE /api/queue/{id}`, `DELETE /api/queue` | profil | |

## Extensions

| Route | Accès | Notes |
|---|---|---|
| `GET /api/extensions`, `GET /api/extensions/{id}` | profil | permissions demandées / accordées, statut (`ok`, `crashed`, `stopped`) |
| `GET /api/extensions/catalog` | adulte | paquets du catalogue local (`extensions/`) |
| `POST /api/extensions/inspect` | adulte | `{"path"}` : ce que demande un paquet avant de l'accepter |
| `POST /api/extensions` | adulte | `{"path", "accept_permissions": [...]}` (dossier ou `.zip`) → `201` |
| `PATCH /api/extensions/{id}` | adulte | `{"enabled"?, "settings"?}` |
| `POST /api/extensions/{id}/update` | adulte | nouvelle version (réglages gardés) |
| `DELETE /api/extensions/{id}` | adulte | `204` |

Permissions : `network`, `library.read`, `library.write`, `playback.control`,
`playback.events`, `storage`, `ui.home`. Une permission non acceptée est refusée.

## Temps réel

`GET /api/ws` (WebSocket). Le serveur envoie `{"type": ..., "data": ...}` :

| `type` | Quand |
|---|---|
| `player.state` | à l'ouverture, puis à chaque changement du lecteur |
| `player.command` | instruction pour l'app TV (`load`, `play`, `pause`, `seek`, `volume`...) |
| `queue.changed` | file modifiée |
| `input` | touche de la télécommande (pour l'app TV) |
| `profile.changed`, `preferences.changed` | profil actif ou préférences (thème à chaud) |
| `pairing.completed` | un téléphone vient d'être appairé |
| `library.updated`, `source.status` | fin de scan, source en ligne / hors ligne |
| `extension.status` | extension installée, arrêtée après un plantage... |

Le client peut envoyer `{"type": "ping"}` (réponse `pong`),
`{"type": "input", "key": "up"}` ou
`{"type": "command", "command": {"action": "pause"}}`.

## Codes d'erreur

Ceux de `fixtures/README.md`, plus :

| Code | HTTP | Quand |
|---|---|---|
| `starting` | 503 | base pas encore prête |
| `localhost_only` | 403 | route réservée à l'app TV |
| `admin_required` | 403 | action réservée à un compte non restreint |
| `route_not_found` | 404 | route inconnue |
| `poster_not_found`, `source_not_found`, `playlist_not_found`, `playlist_item_not_found`, `queue_item_not_found`, `extension_not_found`, `device_not_found` | 404 | |
| `playlist_name_taken`, `source_exists`, `extension_already_installed`, `extension_not_newer` | 409 | |
| `nothing_to_play`, `scan_in_progress` | 409 | |
| `extension_invalid` | 422 | manifeste ou paquet invalide |
| `provider_unavailable` | 503 | fournisseur d'informations injoignable (hors ligne) |
| `payload_too_large` | 413 | fichier envoyé trop gros |
| `internal_error` | 500 | erreur interne (détail dans les logs du serveur) |
