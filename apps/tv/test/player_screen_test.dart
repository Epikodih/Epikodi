// Tests de l'écran de lecture avec un faux moteur (sans libmpv).
import 'package:epikodi_tv/player/playback_state.dart';
import 'package:epikodi_tv/player/player_screen.dart';
import 'package:epikodi_tv/player/time_format.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_engine.dart';
import 'support/harness.dart';

const _source =
    PlaybackSource(uri: 'http://tv.test/stream', title: 'Big Buck Bunny');
const _hide = Duration(seconds: 5);

String? get _focusedLabel => FocusManager.instance.primaryFocus?.debugLabel;

void main() {
  group('formatPlaybackTime', () {
    test('minutes et secondes, heures si besoin', () {
      expect(formatPlaybackTime(Duration.zero), '0:00');
      expect(
          formatPlaybackTime(const Duration(minutes: 5, seconds: 3)), '5:03');
      expect(
          formatPlaybackTime(const Duration(hours: 1, minutes: 5, seconds: 3)),
          '1:05:03');
      expect(formatPlaybackTime(const Duration(seconds: -4)), '0:00');
    });
  });

  group('PlayerScreen', () {
    late FakeEngine engine;
    setUp(() => engine = FakeEngine());

    Future<void> pumpPlayer(WidgetTester tester) async {
      await pumpTv(tester, PlayerScreen(engine: engine, source: _source));
      await tester.pump();
    }

    testWidgets('ouvre la source, affiche le titre et donne le focus à Pause',
        (tester) async {
      await pumpPlayer(tester);
      expect(engine.opened?.uri, _source.uri);
      expect(find.text('Big Buck Bunny'), findsOneWidget);
      expect(semanticsLabel(fr.playerPause), findsOneWidget);
      expect(_focusedLabel, 'player.play');
      await unmount(tester);
      expect(engine.disposed, isTrue, reason: 'moteur libéré à la fermeture');
    });

    testWidgets(
        'Entrée sur le bouton met en pause, état écrit en toutes lettres',
        (tester) async {
      await pumpPlayer(tester);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();
      expect(engine.calls, contains('pause'));
      expect(find.text(fr.playerPaused), findsOneWidget);
      expect(semanticsLabel(fr.playerPlay), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('les contrôles se masquent pendant la lecture, pas en pause',
        (tester) async {
      await pumpPlayer(tester);
      await tester.pump(_hide);
      await tester.pump();
      expect(find.text('Big Buck Bunny'), findsNothing);

      // En pause : ils réapparaissent et restent.
      await engine.pause();
      await tester.pump();
      await tester.pump(_hide * 2);
      expect(find.text('Big Buck Bunny'), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('contrôles masqués : → avance de 30 s et les réaffiche',
        (tester) async {
      await pumpPlayer(tester);
      await tester.pump(_hide);
      await tester.pump();

      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pump();
      expect(engine.calls, contains('seek ${PlayerScreen.forwardStep}'));
      expect(find.text('Big Buck Bunny'), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('contrôles masqués : Entrée bascule lecture / pause',
        (tester) async {
      await pumpPlayer(tester);
      await tester.pump(_hide);
      await tester.pump();

      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();
      expect(engine.calls.last, 'pause');
      await unmount(tester);
    });

    testWidgets('les flèches déplacent le focus entre les boutons',
        (tester) async {
      await pumpPlayer(tester);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pump();
      expect(_focusedLabel, isNot('player.play'));
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();
      expect(engine.calls, contains('seek ${PlayerScreen.forwardStep}'));
      await unmount(tester);
    });

    testWidgets('Retour ferme le lecteur', (tester) async {
      await pumpTv(tester, const Text('médiathèque'));
      final nav = tester.state<NavigatorState>(find.byType(Navigator));
      nav.push(MaterialPageRoute<void>(
          builder: (_) => PlayerScreen(engine: engine, source: _source)));
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(find.byType(PlayerScreen), findsOneWidget);

      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(find.byType(PlayerScreen), findsNothing);
      expect(engine.disposed, isTrue);
      await unmount(tester);
    });

    testWidgets("erreur : message annoncé et contrôles visibles",
        (tester) async {
      await pumpPlayer(tester);
      engine
          .emit((s) => s.copyWith(playing: false, error: 'fichier illisible'));
      await tester.pump();
      await tester.pump(_hide);
      expect(find.text(fr.playerError('fichier illisible')), findsOneWidget);
      expect(find.text('Big Buck Bunny'), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('animations désactivées : chargement sans roue qui tourne',
        (tester) async {
      await pumpPlayer(tester);
      engine.emit((s) => s.copyWith(buffering: true));
      await tester.pump();
      expect(find.text(fr.playerLoading), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      await unmount(tester);
    });

    testWidgets('position lue par le lecteur d\'écran', (tester) async {
      await pumpPlayer(tester);
      engine.emit((s) => s.copyWith(position: const Duration(minutes: 2)));
      await tester.pump();
      final node =
          tester.widget<Semantics>(semanticsLabel(fr.playerPositionLabel));
      expect(node.properties.value, fr.playerPosition('2:00', '10:00'));
      await unmount(tester);
    });

    testWidgets('fenêtre minimale : pas de débordement', (tester) async {
      await pumpTv(tester, PlayerScreen(engine: engine, source: _source),
          size: const Size(960, 540));
      await tester.pump();
      expect(tester.takeException(), isNull);
      await unmount(tester);
    });
  });
}
