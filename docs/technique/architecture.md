# Architecture

> Schéma des composants (interface, médiathèque, lecture, extensions, API, mobile), flux de données, choix de stack justifiés.

La vue d'ensemble du monorepo est dans [ARCHITECTURE.md](../../ARCHITECTURE.md).

## Composants de l'app TV

| Composant | Dossier | Rôle |
|---|---|---|
| Lecture | `apps/tv/lib/player/` | moteur `PlaybackEngine` (media_kit) et écran de lecture, voir [Lecture](lecture.md) |

## Flux de lecture

```
Médiathèque (TV)          Lecteur (TV)                 Cœur Rust
      │  Entrée sur un film     │                            │
      ├────────────────────────►│ PlayerScreen               │
      │                         │ PlaybackEngine.open()      │
      │                         ├── GET /api/media/{id}/stream ──►│
      │                         │   Authorization + Range    │
      │                         │◄── 206 Partial Content ────┤
      │◄──── Retour ────────────┤ moteur libéré              │
```

L'app TV ne lit jamais le disque elle-même : tout fichier passe par le flux du cœur, qui applique le filtre parental.

