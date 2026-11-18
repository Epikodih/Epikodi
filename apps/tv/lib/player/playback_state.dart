import 'package:flutter/foundation.dart';

/// Ce que le lecteur doit ouvrir : une URL (flux HTTP du cœur, fichier local)
/// et les en-têtes à envoyer avec (token Bearer pour le flux du cœur).
@immutable
class PlaybackSource {
  const PlaybackSource({
    required this.uri,
    required this.title,
    this.headers = const {},
  });

  /// Adresse lue par le moteur (`http://…/stream`, `file:///…`).
  final String uri;

  /// Titre affiché et annoncé par le lecteur d'écran.
  final String title;

  /// En-têtes HTTP (ex. `Authorization: Bearer …`). Jamais journalisés.
  final Map<String, String> headers;
}

/// Une piste audio ou de sous-titres, indépendante du moteur de lecture.
@immutable
class MediaTrack {
  const MediaTrack({
    required this.id,
    this.title,
    this.language,
    this.isAudioDescription = false,
  });

  /// Identifiant propre au moteur (ex. `1`, `2` pour mpv).
  final String id;

  /// Titre lu dans le fichier (ex. « Français 5.1 »), peut être absent.
  final String? title;

  /// Code de langue (ex. `fre`, `fr`, `eng`), peut être absent.
  final String? language;

  /// Piste d'audiodescription (pour les personnes aveugles ou malvoyantes).
  final bool isAudioDescription;

  // Mots qui signalent une audiodescription dans le titre d'une piste.
  static final _audioDescriptionPattern = RegExp(
    r'audio[\s-]?description|audiodescri|descriptive|\bAD\b',
    caseSensitive: false,
  );

  /// Vrai si [title] désigne une piste d'audiodescription.
  static bool looksLikeAudioDescription(String? title) =>
      title != null && _audioDescriptionPattern.hasMatch(title);

  @override
  bool operator ==(Object other) => other is MediaTrack && other.id == id;

  @override
  int get hashCode => id.hashCode;

  @override
  String toString() => 'MediaTrack($id, $title, $language)';
}

/// Instantané de l'état du lecteur, publié par le moteur à chaque changement.
/// Immuable : on en crée un nouveau avec [copyWith].
@immutable
class PlaybackState {
  const PlaybackState({
    this.playing = false,
    this.buffering = false,
    this.completed = false,
    this.position = Duration.zero,
    this.duration = Duration.zero,
    this.audioTracks = const [],
    this.subtitleTracks = const [],
    this.audioTrack,
    this.subtitleTrack,
    this.error,
  });

  final bool playing;
  final bool buffering;

  /// Fin du média atteinte.
  final bool completed;
  final Duration position;

  /// Durée totale (zéro tant qu'elle n'est pas connue).
  final Duration duration;

  final List<MediaTrack> audioTracks;
  final List<MediaTrack> subtitleTracks;

  /// Piste audio active (null : choix automatique du moteur).
  final MediaTrack? audioTrack;

  /// Sous-titres actifs (null : désactivés).
  final MediaTrack? subtitleTrack;

  /// Dernière erreur du moteur (fichier illisible, flux coupé…).
  final String? error;

  // Valeur sentinelle : permet à copyWith de remettre un champ à null.
  static const _keep = Object();

  PlaybackState copyWith({
    bool? playing,
    bool? buffering,
    bool? completed,
    Duration? position,
    Duration? duration,
    List<MediaTrack>? audioTracks,
    List<MediaTrack>? subtitleTracks,
    Object? audioTrack = _keep,
    Object? subtitleTrack = _keep,
    Object? error = _keep,
  }) =>
      PlaybackState(
        playing: playing ?? this.playing,
        buffering: buffering ?? this.buffering,
        completed: completed ?? this.completed,
        position: position ?? this.position,
        duration: duration ?? this.duration,
        audioTracks: audioTracks ?? this.audioTracks,
        subtitleTracks: subtitleTracks ?? this.subtitleTracks,
        audioTrack: identical(audioTrack, _keep)
            ? this.audioTrack
            : audioTrack as MediaTrack?,
        subtitleTrack: identical(subtitleTrack, _keep)
            ? this.subtitleTrack
            : subtitleTrack as MediaTrack?,
        error: identical(error, _keep) ? this.error : error as String?,
      );
}
