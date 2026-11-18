# Lecture

> Moteur de lecture retenu, formats supportés, sous-titres, reprise de lecture.

## Moteur de lecture

L'app TV lit les médias avec **media_kit** (libmpv), choix justifié dans l'[ADR 0001](../adr/0001-moteur-de-lecture.md).

Le moteur est isolé derrière l'interface `PlaybackEngine` (`apps/tv/lib/features/player/`). Les écrans ne dépendent que de cette interface, ce qui permet :

- de remplacer media_kit (par VLC par exemple) sans modifier l'interface utilisateur ;
- de tester l'écran de lecture avec un faux moteur, sans libmpv.

## Formats supportés

media_kit s'appuie sur les décodeurs de FFmpeg.

| Catégorie | Formats |
|---|---|
| Conteneurs | MKV, MP4, AVI, MOV, WebM, TS / M2TS, OGG |
| Vidéo | H.264, H.265 / HEVC, AV1, VP9, MPEG-2 (décodage matériel si disponible) |
| Audio | AAC, MP3, FLAC, Opus, AC3 / E-AC3, DTS, TrueHD |
| Sous-titres | SRT, ASS / SSA, PGS, VobSub, intégrés ou externes |

Sous Linux, la liste exacte dépend du FFmpeg de la distribution (`libmpv` système).

## Sous-titres

_À compléter (#22)._

## Reprise de lecture

_À compléter (#24)._
