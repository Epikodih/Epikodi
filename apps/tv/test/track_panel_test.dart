// Tests du panneau « Audio et sous-titres » et du style des sous-titres.
import 'package:epikodi_tv/player/playback_state.dart';
import 'package:epikodi_tv/player/player_screen.dart';
import 'package:epikodi_tv/player/subtitle_style.dart';
import 'package:epikodi_tv/player/track_panel.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_engine.dart';
import 'support/harness.dart';

const _source = PlaybackSource(uri: 'http://tv.test/stream', title: 'Film');

const _fr = MediaTrack(id: '1', title: 'Français 5.1', language: 'fre');
const _ad = MediaTrack(
    id: '2', title: 'Français', language: 'fre', isAudioDescription: true);
const _subFr = MediaTrack(id: '3', title: 'Forcés', language: 'fre');

/// Libellé lu par le lecteur d'écran pour un choix du panneau.
String _choice(String prefix, String text, {bool selected = false}) =>
    '$prefix : $text, ${selected ? 'sélectionné' : 'non sélectionné'}';

void main() {
  group('trackLabel', () {
    test('titre et langue du fichier', () {
      expect(trackLabel(_fr, 0, fr), 'Français 5.1 · FRE');
    });

    test('sans titre ni langue : « Piste N »', () {
      expect(trackLabel(const MediaTrack(id: '9'), 1, fr), 'Piste 2');
    });

    test("l'audiodescription est écrite en toutes lettres", () {
      expect(trackLabel(_ad, 1, fr), 'Français · FRE · Audiodescription');
      // Pas de doublon si le titre le dit déjà.
      const titled = MediaTrack(
          id: '4', title: 'Audiodescription', isAudioDescription: true);
      expect(trackLabel(titled, 0, fr), 'Audiodescription');
    });
  });

  group('SubtitleStyle', () {
    test('taille, couleur et fond appliqués au texte', () {
      const s = SubtitleStyle(
          size: SubtitleSize.large,
          color: SubtitleColor.yellow,
          background: SubtitleBackground.opaque);
      final t = s.toTextStyle();
      expect(t.fontSize, SubtitleStyle.baseFontSize * 1.35);
      expect(t.color, SubtitleColor.yellow.color);
      expect(t.backgroundColor, SubtitleBackground.opaque.color);
      expect(t.shadows, isNull);
    });

    test('sans fond : contour sombre pour rester lisible', () {
      const s = SubtitleStyle(background: SubtitleBackground.none);
      expect(s.toTextStyle().shadows, isNotEmpty);
    });
  });

  group('panneau dans le lecteur', () {
    late FakeEngine engine;
    late List<SubtitleStyle> saved;

    setUp(() {
      engine = FakeEngine();
      saved = [];
    });

    Future<void> openPanel(WidgetTester tester) async {
      await pumpTv(
        tester,
        PlayerScreen(
          engine: engine,
          source: _source,
          onSubtitleStyleChanged: saved.add,
        ),
      );
      await tester.pump();
      engine.emit((s) => s.copyWith(
            audioTracks: [_fr, _ad],
            subtitleTracks: [_subFr],
            audioTrack: _fr,
          ));
      await tester.pump();
      await tester.tap(find.byIcon(Icons.subtitles_outlined));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
    }

    testWidgets('liste les pistes, la piste active est cochée', (tester) async {
      await openPanel(tester);
      expect(find.byType(TrackPanel), findsOneWidget);
      expect(
          semanticsLabel(
              _choice('Audio', 'Français 5.1 · FRE', selected: true)),
          findsOneWidget);
      expect(
          semanticsLabel(_choice('Audio', 'Français · FRE · Audiodescription')),
          findsOneWidget);
      expect(
          semanticsLabel(_choice('Sous-titres', 'Désactivés', selected: true)),
          findsOneWidget);
      await unmount(tester);
    });

    testWidgets('le premier choix a le focus, Entrée le sélectionne',
        (tester) async {
      await openPanel(tester);
      // Pistes une par ligne : ↓ passe à l'audiodescription.
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();
      expect(engine.calls, contains('audio 2'), reason: 'audiodescription');
      await unmount(tester);
    });

    testWidgets('choisir des sous-titres puis les désactiver', (tester) async {
      await openPanel(tester);
      await tester.tap(find.text('Forcés · FRE'));
      await tester.pump();
      expect(engine.state.value.subtitleTrack, _subFr);

      await tester.tap(find.text('Désactivés'));
      await tester.pump();
      expect(engine.calls.last, 'subtitle null');
      await unmount(tester);
    });

    testWidgets('le style choisi est appliqué à la vidéo et transmis',
        (tester) async {
      await openPanel(tester);
      await tester.ensureVisible(find.text('Grande'));
      await tester.tap(find.text('Grande'));
      await tester.pump();
      await tester.ensureVisible(find.text('Jaune'));
      await tester.tap(find.text('Jaune'));
      await tester.pump();

      expect(engine.subtitleStyle?.size, SubtitleSize.large);
      expect(engine.subtitleStyle?.color, SubtitleColor.yellow);
      expect(
          saved.last,
          const SubtitleStyle(
              size: SubtitleSize.large, color: SubtitleColor.yellow));
      await unmount(tester);
    });

    testWidgets('Retour ferme le panneau, le focus revient sur Lecture / Pause',
        (tester) async {
      await openPanel(tester);
      // Panneau ouvert : les contrôles ne se masquent pas.
      await tester.pump(const Duration(seconds: 10));
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.byType(TrackPanel), findsNothing);
      expect(find.byType(PlayerScreen), findsOneWidget);
      expect(FocusManager.instance.primaryFocus?.debugLabel, 'player.play');
      await unmount(tester);
    });

    testWidgets('fichier sans piste audio : message explicite', (tester) async {
      await pumpTv(tester, PlayerScreen(engine: engine, source: _source));
      await tester.pump();
      await tester.tap(find.byIcon(Icons.subtitles_outlined));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text(fr.playerNoTrack), findsOneWidget);
      await unmount(tester);
    });
  });
}
