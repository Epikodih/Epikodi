# ADR 0001 : moteur de lecture de l'app TV

- **Statut** : accepté
- **Date** : 2026-11-18
- **Issue** : #18 (parent #10)

## Contexte

L'app TV (Flutter, Linux / Windows / macOS) doit lire les vidéos et la musique de la médiathèque : conteneurs MKV, MP4, AVI, MOV, TS, codecs H.264, HEVC, AV1, VP9, audio AAC, FLAC, AC3, DTS, sous-titres SRT, ASS, PGS et VobSub. Elle doit aussi permettre de choisir la piste audio (audiodescription comprise) et les sous-titres, et régler leur style.

## Options étudiées

| Option | Plateformes desktop | Formats | Points faibles |
|---|---|---|---|
| `flutter_vlc_player` (libVLC) | aucune (Android, iOS seulement) | FFmpeg | inutilisable pour l'app TV |
| `dart_vlc` (libVLC) | Linux, Windows | FFmpeg | abandonné, pas de macOS |
| `vlc_player` (libVLC) | Linux, Windows, macOS | FFmpeg | très jeune (desktop ajouté en 2026), libVLC système sous Linux, téléchargement au build sous Windows |
| **`media_kit` (libmpv)** | Linux, Windows, macOS | FFmpeg | DVD / Blu-ray lus sans les menus |

VLC et mpv s'appuient tous deux sur les décodeurs de FFmpeg : la couverture des formats est équivalente.

## Décision

On utilise **media_kit** (`media_kit`, `media_kit_video`, `media_kit_libs_video`), déjà déclaré dans `apps/tv/pubspec.yaml` et installé par la CI (`libmpv-dev`).

Le moteur est caché derrière une interface `PlaybackEngine` (dossier `apps/tv/lib/features/player/`). Le reste de l'app ne connaît que cette interface.

## Conséquences

- Un moteur VLC pourra être ajouté plus tard en implémentant `PlaybackEngine`, sans toucher aux écrans.
- Les tests de l'écran de lecture utilisent un faux moteur : ils tournent sans libmpv.
- Sous Linux, les formats lus dépendent du FFmpeg de la distribution (même contrainte qu'avec VLC). Sous Windows et macOS, media_kit embarque ses bibliothèques.
- Les photos (HEIC, etc.) ne relèvent pas du lecteur : elles seront traitées à part si le besoin est retenu.
