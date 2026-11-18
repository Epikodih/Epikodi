import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'playback_state.dart';
import 'subtitle_style.dart';

/// Moteur de lecture vu par l'app TV (voir docs/adr/0001-moteur-de-lecture.md).
///
/// Les écrans ne dépendent que de cette interface : l'implémentation
/// media_kit ([MediaKitEngine]) peut être remplacée (VLC, faux moteur pour
/// les tests) sans toucher à l'interface utilisateur.
abstract class PlaybackEngine {
  /// État courant, mis à jour par le moteur (position, pistes, erreurs…).
  ValueListenable<PlaybackState> get state;

  /// Ouvre [source] et lance la lecture.
  Future<void> open(PlaybackSource source);

  Future<void> play();
  Future<void> pause();

  /// Bascule lecture / pause.
  Future<void> togglePlay() => state.value.playing ? pause() : play();

  /// Va à [position], bornée entre zéro et la durée du média.
  Future<void> seek(Duration position);

  /// Choisit la piste audio.
  Future<void> setAudioTrack(MediaTrack track);

  /// Choisit les sous-titres, ou les désactive avec `null`.
  Future<void> setSubtitleTrack(MediaTrack? track);

  /// Surface vidéo (sans contrôles : l'écran de lecture dessine les siens),
  /// avec les sous-titres dessinés selon [subtitleStyle].
  Widget buildVideo(
    BuildContext context, {
    SubtitleStyle subtitleStyle = const SubtitleStyle(),
  });

  /// Libère le moteur (fichier, décodeurs, fenêtre vidéo).
  Future<void> dispose();
}
