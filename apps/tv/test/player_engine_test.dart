// Tests du modèle de lecture et de la conversion des pistes media_kit.
// Aucun libmpv n'est chargé : on ne manipule que des objets de données.
import 'package:epikodi_tv/player/media_kit_engine.dart';
import 'package:epikodi_tv/player/playback_state.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:media_kit/media_kit.dart';

void main() {
  group('MediaTrack.looksLikeAudioDescription', () {
    test('reconnaît les libellés courants', () {
      for (final title in [
        'Audiodescription',
        'Français (audio description)',
        'English - Audio-Description',
        'Descriptive audio',
        'FR AD',
      ]) {
        expect(MediaTrack.looksLikeAudioDescription(title), isTrue,
            reason: title);
      }
    });

    test('ignore les pistes ordinaires', () {
      for (final title in [null, 'Français 5.1', 'Commentaires', 'Radio']) {
        expect(MediaTrack.looksLikeAudioDescription(title), isFalse,
            reason: '$title');
      }
    });
  });

  group('PlaybackState.copyWith', () {
    const fr = MediaTrack(id: '1', language: 'fre');

    test('garde les champs non fournis', () {
      final s = const PlaybackState(subtitleTrack: fr, error: 'x')
          .copyWith(playing: true);
      expect(s.playing, isTrue);
      expect(s.subtitleTrack, fr);
      expect(s.error, 'x');
    });

    test('remet un champ nullable à null', () {
      final s = const PlaybackState(subtitleTrack: fr, error: 'x')
          .copyWith(subtitleTrack: null, error: null);
      expect(s.subtitleTrack, isNull);
      expect(s.error, isNull);
    });
  });

  group('conversion des pistes media_kit', () {
    test('realTracks retire les pistes virtuelles auto et no', () {
      final tracks = [
        AudioTrack.auto(),
        AudioTrack.no(),
        const AudioTrack('1', 'Français', 'fre'),
      ];
      expect(realTracks(tracks).map((t) => t.id), ['1']);
    });

    test('toMediaTrack copie id, titre et langue', () {
      final t = toMediaTrack(const SubtitleTrack('3', 'Forcés', 'fre'));
      expect((t.id, t.title, t.language), ('3', 'Forcés', 'fre'));
      expect(t.isAudioDescription, isFalse);
    });

    test("toMediaTrack détecte l'audiodescription sur l'audio seulement", () {
      expect(
          toMediaTrack(const AudioTrack('2', 'Audiodescription', 'fre'))
              .isAudioDescription,
          isTrue);
      expect(
          toMediaTrack(const SubtitleTrack('2', 'Audiodescription', 'fre'))
              .isAudioDescription,
          isFalse);
    });
  });
}
