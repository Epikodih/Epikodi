# Lecture

> Moteur de lecture retenu, formats supportés, sous-titres, reprise de lecture.

## Moteur de lecture

L'app TV lit les médias avec **media_kit** (libmpv), choix justifié dans l'[ADR 0001](../adr/0001-moteur-de-lecture.md).

Le moteur est isolé derrière l'interface `PlaybackEngine` (`apps/tv/lib/player/`). Les écrans ne dépendent que de cette interface, ce qui permet :

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

## Écran de lecture

`PlayerScreen` (`apps/tv/lib/player/player_screen.dart`) affiche la vidéo en plein écran et un bandeau de contrôles en bas : titre, barre de progression avec les temps écrits, boutons Reculer 10 s, Lecture / Pause, Avancer 30 s (#21).

### Touches

| Touche | Contrôles masqués | Contrôles affichés |
|---|---|---|
| Entrée / OK / Espace | lecture ↔ pause | active le bouton qui a le focus |
| ← / → | recule de 10 s / avance de 30 s | déplace le focus entre les boutons |
| ↑ / ↓ | affiche les contrôles | déplace le focus |
| Retour / Échap | quitte la lecture | quitte la lecture |
| Touches multimédia | lecture, pause, avance, retour rapide | idem |

Les contrôles se masquent après 5 s sans touche, seulement pendant la lecture. En pause, en erreur ou à la fin du média, ils restent affichés.

### Accessibilité

- **Focus** : au lancement, il est sur Lecture / Pause. Contour blanc de 4 px du design system, visible à 3 m.
- **Lecteur d'écran** : chaque bouton a un libellé (« Pause », « Avancer de 30 secondes »). La barre annonce « Position de lecture : 2:00 sur 10:00 ». Chargement, erreur et fin de lecture sont des zones annoncées automatiquement (`liveRegion`). La vidéo elle-même est exclue de l'arbre sémantique.
- **Pas seulement la couleur** : l'état « En pause » est écrit en toutes lettres, les temps sont affichés à côté de la barre.
- **Animations désactivées** (`reducedMotion`) : pas de fondu des contrôles, pas de roue de chargement (le texte « Chargement… » suffit). Aucun contenu clignotant.
- **Contraste** : texte blanc sur dégradé noir à 90 % en bas de l'image.

### Tests

`apps/tv/test/player_screen_test.dart` utilise un faux moteur (`test/support/fake_engine.dart`) : touches, masquage des contrôles, focus, Retour, erreurs, annonces du lecteur d'écran, fenêtre minimale.

## Sous-titres

_À compléter (#22)._

## Reprise de lecture

_À compléter (#24)._
