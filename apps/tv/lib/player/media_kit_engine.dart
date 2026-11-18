import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

import 'playback_engine.dart';
import 'playback_state.dart';

/// Implémentation de [PlaybackEngine] avec media_kit (libmpv).
///
/// Les flux (streams) du `Player` sont regroupés dans un seul
/// [PlaybackState] : l'écran n'a qu'une valeur à écouter.
class MediaKitEngine extends PlaybackEngine {
  MediaKitEngine() : _player = Player() {
    _video = VideoController(_player);
    final s = _player.stream;
    _subscriptions = [
      s.playing.listen((v) => _update((st) => st.copyWith(playing: v))),
      s.buffering.listen((v) => _update((st) => st.copyWith(buffering: v))),
      s.completed.listen((v) => _update((st) => st.copyWith(completed: v))),
      s.position.listen((v) => _update((st) => st.copyWith(position: v))),
      s.duration.listen((v) => _update((st) => st.copyWith(duration: v))),
      s.tracks.listen(_onTracks),
      s.track.listen(_onSelectedTrack),
      s.error.listen((v) => _update((st) => st.copyWith(error: v))),
    ];
  }

  final Player _player;
  late final VideoController _video;
  late final List<StreamSubscription<Object?>> _subscriptions;
  final _state = ValueNotifier(const PlaybackState());
  bool _disposed = false;

  // Pistes media_kit d'origine, retrouvées par identifiant pour les sélectionner.
  Map<String, AudioTrack> _audioById = const {};
  Map<String, SubtitleTrack> _subtitleById = const {};

  @override
  ValueListenable<PlaybackState> get state => _state;

  void _update(PlaybackState Function(PlaybackState) change) {
    if (!_disposed) _state.value = change(_state.value);
  }

  void _onTracks(Tracks tracks) {
    _audioById = {for (final t in realTracks(tracks.audio)) t.id: t};
    _subtitleById = {for (final t in realTracks(tracks.subtitle)) t.id: t};
    _update((st) => st.copyWith(
          audioTracks: _audioById.values.map(toMediaTrack).toList(),
          subtitleTracks: _subtitleById.values.map(toMediaTrack).toList(),
        ));
  }

  void _onSelectedTrack(Track track) {
    final audio = _audioById[track.audio.id];
    final subtitle = _subtitleById[track.subtitle.id];
    _update((st) => st.copyWith(
          audioTrack: audio == null ? null : toMediaTrack(audio),
          subtitleTrack: subtitle == null ? null : toMediaTrack(subtitle),
        ));
  }

  @override
  Future<void> open(PlaybackSource source) async {
    // Nouveau média : on repart d'un état vierge (pas d'ancienne erreur).
    _update((_) => const PlaybackState());
    await _player.open(Media(source.uri, httpHeaders: source.headers));
  }

  @override
  Future<void> play() => _player.play();

  @override
  Future<void> pause() => _player.pause();

  @override
  Future<void> seek(Duration position) {
    final max = _state.value.duration;
    final target = position < Duration.zero
        ? Duration.zero
        : (max > Duration.zero && position > max ? max : position);
    return _player.seek(target);
  }

  @override
  Future<void> setAudioTrack(MediaTrack track) async {
    final t = _audioById[track.id];
    if (t != null) await _player.setAudioTrack(t);
  }

  @override
  Future<void> setSubtitleTrack(MediaTrack? track) async {
    final t = track == null ? SubtitleTrack.no() : _subtitleById[track.id];
    if (t != null) await _player.setSubtitleTrack(t);
  }

  @override
  Widget buildVideo(BuildContext context) => Video(
        controller: _video,
        controls: NoVideoControls,
        fill: Colors.black,
      );

  @override
  Future<void> dispose() async {
    _disposed = true;
    for (final s in _subscriptions) {
      await s.cancel();
    }
    _state.dispose();
    await _player.dispose();
  }
}

/// Retire les pistes virtuelles de mpv (`auto` : choix automatique,
/// `no` : désactivée) pour ne garder que celles du fichier.
@visibleForTesting
List<T> realTracks<T extends Object>(List<T> tracks) => tracks.where((t) {
      final id = switch (t) {
        AudioTrack(:final id) => id,
        SubtitleTrack(:final id) => id,
        _ => '',
      };
      return id != 'auto' && id != 'no';
    }).toList();

/// Convertit une piste media_kit en [MediaTrack] (indépendante du moteur).
@visibleForTesting
MediaTrack toMediaTrack(Object track) {
  final (id, title, language) = switch (track) {
    AudioTrack t => (t.id, t.title, t.language),
    SubtitleTrack t => (t.id, t.title, t.language),
    _ => throw ArgumentError.value(track, 'track', 'piste inconnue'),
  };
  return MediaTrack(
    id: id,
    title: title,
    language: language,
    // L'audiodescription ne concerne que les pistes audio.
    isAudioDescription:
        track is AudioTrack && MediaTrack.looksLikeAudioDescription(title),
  );
}
