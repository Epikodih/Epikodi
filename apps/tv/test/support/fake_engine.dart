// Faux moteur de lecture : aucun libmpv, le test pilote l'état à la main.
import 'package:epikodi_tv/player/playback_engine.dart';
import 'package:epikodi_tv/player/playback_state.dart';
import 'package:epikodi_tv/player/subtitle_style.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

class FakeEngine extends PlaybackEngine {
  final _state = ValueNotifier(const PlaybackState());

  /// Appels reçus, dans l'ordre (ex. `open`, `pause`, `seek 0:00:30.000000`).
  final calls = <String>[];
  PlaybackSource? opened;
  bool disposed = false;

  /// Dernier style de sous-titres reçu par la surface vidéo.
  SubtitleStyle? subtitleStyle;

  @override
  ValueListenable<PlaybackState> get state => _state;

  /// Change l'état comme le ferait le vrai moteur.
  void emit(PlaybackState Function(PlaybackState) change) =>
      _state.value = change(_state.value);

  @override
  Future<void> open(PlaybackSource source) async {
    calls.add('open');
    opened = source;
    emit((s) =>
        s.copyWith(playing: true, duration: const Duration(minutes: 10)));
  }

  @override
  Future<void> play() async {
    calls.add('play');
    emit((s) => s.copyWith(playing: true));
  }

  @override
  Future<void> pause() async {
    calls.add('pause');
    emit((s) => s.copyWith(playing: false));
  }

  @override
  Future<void> seek(Duration position) async {
    calls.add('seek $position');
    emit((s) => s.copyWith(position: position));
  }

  @override
  Future<void> setAudioTrack(MediaTrack track) async {
    calls.add('audio ${track.id}');
    emit((s) => s.copyWith(audioTrack: track));
  }

  @override
  Future<void> setSubtitleTrack(MediaTrack? track) async {
    calls.add('subtitle ${track?.id}');
    emit((s) => s.copyWith(subtitleTrack: track));
  }

  @override
  Widget buildVideo(
    BuildContext context, {
    SubtitleStyle subtitleStyle = const SubtitleStyle(),
  }) {
    this.subtitleStyle = subtitleStyle;
    return const SizedBox.expand();
  }

  @override
  Future<void> dispose() async => disposed = true;
}
