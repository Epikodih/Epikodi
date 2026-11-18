# Architecture

> Schéma des composants (interface, médiathèque, lecture, extensions, API, mobile), flux de données, choix de stack justifiés.

La vue d'ensemble du monorepo est dans [ARCHITECTURE.md](../../ARCHITECTURE.md).

## Composants de l'app TV

| Composant | Dossier | Rôle |
|---|---|---|
| Lecture | `apps/tv/lib/features/player/` | moteur `PlaybackEngine` (media_kit) et écran de lecture, voir [Lecture](lecture.md) |
