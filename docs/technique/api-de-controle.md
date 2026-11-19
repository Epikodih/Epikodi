# API de contrôle

> Référence des commandes et événements, appairage, découverte réseau, avec exemples.

Le contrat de référence est [fixtures/routes.json](../../fixtures/routes.json), décrit dans [fixtures/README.md](../../fixtures/README.md).

## Flux de lecture

`GET /api/media/{id}/stream` sert le fichier d'un média lisible (film, épisode, morceau, épisode de podcast) au lecteur de l'app TV (#20).

| Point | Règle |
|---|---|
| Authentification | `Authorization: Bearer <token>` dans l'en-tête, jamais dans l'URL |
| Avance rapide | `Range: bytes=…` → `206 Partial Content` + `Content-Range` |
| Filtre parental | un média invisible pour le profil répond `404 media_not_found` |
| Fichier absent | `410 media_unavailable` (NAS débranché, fichier supprimé) |
| Conteneur | `422 not_playable` (série, saison, artiste, album) |

Côté TV, `Api.streamSource(media)` construit la `PlaybackSource` (URL + en-tête) donnée au moteur.

Exemple :

```http
GET /api/media/1/stream HTTP/1.1
Authorization: Bearer <token>
Range: bytes=1048576-

HTTP/1.1 206 Partial Content
Content-Type: video/x-matroska
Content-Range: bytes 1048576-734003199/734003200
```
