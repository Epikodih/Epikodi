// Ouvrir un média depuis la médiathèque (#23), avec un faux moteur.
import 'package:epikodi_tv/api/api.dart';
import 'package:epikodi_tv/library/library_screen.dart';
import 'package:epikodi_tv/player/player_screen.dart';
import 'package:epikodi_tv/player/subtitle_style.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_engine.dart';
import 'support/harness.dart';

void main() {
  late FixtureBackend backend;
  late List<FakeEngine> engines;

  setUp(() {
    backend = FixtureBackend();
    engines = [];
  });

  Future<void> pumpLibrary(WidgetTester tester) async {
    await pumpTv(
      tester,
      LibraryScreen(
        api: Api(fixtureClient(backend).http),
        profile: Map<String, dynamic>.of(
            readFixture('session/select_response.json')['profile']),
        onChangeProfile: () {},
        onBackToPairing: () {},
        engineFactory: () {
          final e = FakeEngine();
          engines.add(e);
          return e;
        },
      ),
    );
    await settle(tester);
  }

  Future<void> transition(WidgetTester tester) async {
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
  }

  testWidgets('Entrée sur un film ouvre le lecteur sur son flux',
      (tester) async {
    await pumpLibrary(tester);
    await tester.tap(find.text('Big Buck Bunny'));
    await transition(tester);

    expect(find.byType(PlayerScreen), findsOneWidget);
    expect(engines.single.opened?.uri, '$testBaseUrl/api/media/1/stream');
    expect(engines.single.opened?.title, 'Big Buck Bunny');
    await unmount(tester);
  });

  testWidgets('Retour revient à la médiathèque, focus sur la même carte',
      (tester) async {
    await pumpLibrary(tester);
    // Le premier film a le focus au départ : Entrée l'ouvre.
    final before = FocusManager.instance.primaryFocus;
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await transition(tester);
    expect(find.byType(PlayerScreen), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await transition(tester);
    expect(find.byType(PlayerScreen), findsNothing);
    expect(engines.single.disposed, isTrue);
    expect(FocusManager.instance.primaryFocus, before);
    await unmount(tester);
  });

  testWidgets('un média indisponible est signalé et ne se lit pas',
      (tester) async {
    await pumpLibrary(tester);
    expect(semanticsLabel('Elephants Dream, ${fr.mediaUnavailable}'),
        findsOneWidget);
    expect(find.text(fr.mediaUnavailable), findsOneWidget);

    await tester.tap(find.text('Elephants Dream'));
    await transition(tester);
    expect(find.byType(PlayerScreen), findsNothing);
    await unmount(tester);
  });

  testWidgets('une série ne lance pas la lecture', (tester) async {
    await pumpLibrary(tester);
    await tester.ensureVisible(find.text('Série Démo'));
    await tester.tap(find.text('Série Démo'));
    await transition(tester);
    expect(find.byType(PlayerScreen), findsNothing);
    await unmount(tester);
  });

  testWidgets("le style des sous-titres est gardé d'une lecture à l'autre",
      (tester) async {
    await pumpLibrary(tester);
    await tester.tap(find.text('Big Buck Bunny'));
    await transition(tester);
    final player = tester.widget<PlayerScreen>(find.byType(PlayerScreen));
    player.onSubtitleStyleChanged!(
        const SubtitleStyle(size: SubtitleSize.extraLarge));

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await transition(tester);
    await tester.tap(find.text('Big Buck Bunny'));
    await transition(tester);
    expect(tester.widget<PlayerScreen>(find.byType(PlayerScreen)).subtitleStyle,
        const SubtitleStyle(size: SubtitleSize.extraLarge));
    await unmount(tester);
  });
}
