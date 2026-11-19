# Fixtures : API du media center

Chaque fichier fixe la **forme** d'une requête ou d'une réponse (champs et types), pas un scénario. Ils sont partagés par le backend Rust, le backend de test Python et les apps Flutter (TV et télécommande). Si un nom de champ change ici, les tests des deux côtés doivent échouer.

## Utilisation

- **Rust** : un test lit chaque fichier, le décode dans la structure `serde`, le réencode et compare.
- **Flutter** : un test charge le même fichier, appelle `fromJson` puis `toJson` et compare.
- **Backend Python de test** : il lit `routes.json` et sert le fichier `response` de la route demandée. Pour les cas particuliers (erreurs, profil restreint), il adapte le contenu lui-même en suivant les règles ci-dessous.

## Conventions

- Dates en ISO 8601 UTC (`2026-11-07T18:31:12Z`), champs en `snake_case`.
- Valeurs fictives (code `482916`, PIN `1234`, token, adresses).
- Une erreur a toujours la forme de `errors/error.json` : `{"error": {"code", "message", "details"}}`. `details` est facultatif. Les apps se basent sur `code`, jamais sur `message`.

## Routes

| Route | Auth | Requête | Réponse |
|---|---|---|---|
| `GET /api/health` | aucune | | `health/ok.json` |
| `POST /api/tv/connect` (201) | localhost | `tv/connect_request.json` | `tv/connect_response.json` |
| `POST /api/tv/pairing/refresh` | localhost | `tv/refresh_request.json` | `tv/refresh_response.json` |
| `GET /api/tv/status` | localhost | | `tv/status.json` |
| `POST /api/remote/pair` | aucune | `pairing/request.json` | `pairing/response.json` |
| `GET /api/profiles` | Bearer | | `profiles/list.json` |
| `POST /api/profiles` (201) | Bearer | `profiles/create_request.json` | `profiles/create_response.json` |
| `GET /api/profiles/{id}` | Bearer | | `profiles/get_response.json` |
| `PATCH /api/profiles/{id}` | Bearer | `profiles/update_request.json` | `profiles/update_response.json` |
| `GET /api/session` | Bearer | | `session/current.json` |
| `POST /api/session/profile` | Bearer | `session/select_request.json` | `session/select_response.json` |
| `GET /api/media` | Bearer + profil choisi | | `media/list.json` |

`routes.json` reprend ce tableau, avec les codes d'erreur possibles de chaque route.

## Règles que les fixtures ne peuvent pas montrer

**`tv/status`** : `phase` vaut `pairing` (code affiché), `paired` (téléphone appairé, profil pas encore choisi) ou `ready` (profil actif). `pairing` est rempli seulement en phase `pairing`, sinon `null`. `device` reste `null` jusqu'à l'appairage. `active_profile` reste `null` jusqu'au choix du profil.

**Profil** (`profile` dans les réponses) : `kind` vaut `account` ou `guest`. Un invité a `id: null`, `icon: null`, `age: 0` et `is_restricted: true`. Il n'existe pas en base.

**`profiles/create_request`** : `pin` (4 chiffres) est obligatoire si `is_restricted` vaut `false`, et absent sinon. Un invité ne peut pas créer de compte.

**`GET /api/profiles`** : renvoie tout ce qu'il faut pour la page des comptes (invité, droit de créer, liste des comptes).

**`profiles/update_request`** : mêmes champs que la création. `pin` est obligatoire quand on touche à la restriction d'âge (`age` ou `is_restricted`) ; le serveur répond `pin_required` ou `pin_invalid` sinon. Un invité ne peut modifier aucun compte.

**`session/select_request`** :
- `{"type": "guest"}` : ni `profile_id`, ni `pin`
- compte restreint : `profile_id` seul
- compte non restreint : `profile_id` et `pin`

**`session/current`** : `profile` est `null` tant qu'aucun profil n'est choisi.

**`media/list`** : paramètres facultatifs `kind`, `parent_id`, `limit` (50 par défaut), `offset`.
- Sans `parent_id`, c'est le niveau du haut (films, séries, artistes, podcasts). Avec, c'est le contenu de ce média (saisons d'une série, épisodes d'une saison, morceaux d'un album).
- **Le serveur filtre selon le profil actif** : un profil restreint reçoit seulement les médias dont `age_rating` est connu et inférieur ou égal à son âge. L'app ne refait jamais ce filtre.
- `poster_url` est une URL de l'API (pas un chemin du disque), `null` sans affiche. Elle exige le token, donc le chargeur d'images de l'app doit l'envoyer dans l'en-tête.
- `status: "missing"` : le fichier a disparu, le média reste listé pour être affiché grisé.
- `match_status: "unmatched"` : le titre est le nom du fichier, sans affiche ni classification (`age_rating: null`).
- Tri par titre (sans tenir compte de la casse) au niveau du haut, par `sort_number` dans un parent.

## Codes d'erreur

| Code | HTTP | Quand |
|---|---|---|
| `unauthorized` | 401 | Token manquant, expiré ou révoqué : retour à l'écran d'appairage |
| `pairing_code_invalid` | 403 | Code incorrect (`details.attempts_left`) |
| `pairing_code_expired` | 410 | Code expiré, la TV en affiche un nouveau |
| `pairing_too_many_attempts` | 429 | Code invalidé (`details.retry_after_s`) |
| `profile_required` | 403 | Aucun profil choisi |
| `guest_forbidden` | 403 | Un invité tente de créer un compte |
| `pin_required` | 403 | Compte non restreint choisi sans PIN |
| `pin_invalid` | 403 | PIN incorrect (`details.attempts_left`) |
| `pin_locked` | 429 | Trop d'essais (`details.retry_after_s`) |
| `tv_session_not_found` | 404 | Session TV inconnue (serveur redémarré) : la TV refait `POST /api/tv/connect` |
| `profile_not_found` | 404 | Profil inconnu |
| `profile_name_taken` | 409 | Nom déjà pris |
| `validation_failed` | 422 | Champ invalide (`details.fields`) |
| `media_not_found` | 404 | `parent_id` inconnu |
| `invalid_query` | 422 | Paramètre invalide (`details.fields`) |

Seule exception : `GET /api/health` répond `503` avec `{"status": "starting"}` tant que la base n'est pas prête.

## Choix à valider

1. La TV interroge `GET /api/tv/status` à intervalle régulier pour savoir quand l'appairage a réussi (pas de notification poussée).
2. Créer un compte l'active aussitôt (`"active": true`).
3. Le QR est une alternative au code tapé : il contient le code d'appairage (`code`), en plus de l'adresse, du port et de l'identifiant de session. Le téléphone connaît déjà le serveur : scanner suffit pour appairer (`POST /api/remote/pair` avec ce code). Aucun risque de plus : le même code est affiché en clair sous le QR. Le QR change donc à chaque nouveau code. Le schéma `mediacenter://` est provisoire.
4. Le token expire au bout de 30 jours (`expires_at`, `null` s'il n'expire pas).
5. Les médias `missing` sont renvoyés dans la liste.

## Écarts avec `schema.sql` à régler

- **PIN par compte** : le document le prévoit pour chaque compte non restreint. Le schéma n'a qu'un `parental_pin_hash` global dans `app_state` et rien dans `user`. Il faut ajouter `user.pin_hash`, ou revenir à un PIN global.
- **Âge à la création** : `user.age` est obligatoire dans le schéma, mais le document ne demande que le nom et la restriction. Les fixtures contiennent `age`.
- **Appareil** : le document parle d'un `install_id` et d'une expiration du token. `device` n'a ni l'un ni l'autre.
