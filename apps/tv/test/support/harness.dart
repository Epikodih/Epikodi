// Outils communs aux tests de pages de l'app TV.
import 'package:epikodi_client/epikodi_client.dart';
import 'package:epikodi_client/testing.dart';
import 'package:design_system/design_system.dart';
import 'package:epikodi_tv/l10n/app_localizations.dart';
import 'package:epikodi_tv/locale.dart';
import 'package:epikodi_tv/pairing/tv_events.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

export 'package:epikodi_client/testing.dart';

const testBaseUrl = 'http://tv.test';

/// Textes français de l'app (langue des tests).
final fr = lookupAppLocalizations(const Locale('fr'));
final en = lookupAppLocalizations(const Locale('en'));

/// Monte [home] dans la même enveloppe que l'app (thème, langue, réglages
/// d'affichage), sur un écran 1920×1080, animations réduites.
Future<void> pumpTv(
  WidgetTester tester,
  Widget home, {
  Size size = const Size(1920, 1080),
}) async {
  appLocale.value = const Locale('fr');
  tester.view
    ..physicalSize = size
    ..devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  final settings = UiSettings(reducedMotion: true);
  addTearDown(settings.dispose);

  await tester.pumpWidget(ValueListenableBuilder<Locale>(
    valueListenable: appLocale,
    builder: (context, locale, _) => MaterialApp(
      theme: AppTheme.dark(),
      locale: locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) =>
          AppUiScope(settings: settings, child: child!),
      home: home,
    ),
  ));
}

/// Laisse passer les réponses du faux serveur (microtâches) et les rebuilds.
Future<void> settle(WidgetTester tester) async {
  for (var i = 0; i < 5; i++) {
    await tester.pump(const Duration(milliseconds: 20));
  }
}

/// Démonte l'arbre : annule les timers (compte à rebours…) avant la fin du test.
Future<void> unmount(WidgetTester tester) async {
  await tester.pumpWidget(const SizedBox());
  await tester.pump(const Duration(seconds: 1));
}

/// Trouve un widget Semantics par son libellé (sans activer l'arbre sémantique).
Finder semanticsLabel(String label) => find.byWidgetPredicate(
      (w) => w is Semantics && w.properties.label == label,
      description: 'Semantics("$label")',
    );

/// Client branché sur le faux serveur à fixtures.
EpikodiClient fixtureClient(FixtureBackend backend) =>
    EpikodiClient(baseUrl: testBaseUrl, httpClient: backend.client);

/// Faux flux temps réel : le test pousse lui-même les événements.
class FakeTvEvents extends TvEvents {
  FakeTvEvents({
    required super.sessionId,
    required super.onEvent,
    super.onSessionLost,
  }) : super(baseUrl: testBaseUrl);

  bool started = false;
  bool closed = false;

  @override
  void start() => started = true;

  @override
  void close() => closed = true;

  void emit(Map<String, dynamic> data) => onEvent(data);
  void loseSession() => onSessionLost?.call();
}

/// Garde tous les [FakeTvEvents] créés par l'écran.
class FakeEventsHub {
  final created = <FakeTvEvents>[];

  FakeTvEvents get last => created.last;

  TvEvents create({
    required String sessionId,
    required void Function(Map<String, dynamic> data) onEvent,
    void Function()? onSessionLost,
  }) {
    final e = FakeTvEvents(
      sessionId: sessionId,
      onEvent: onEvent,
      onSessionLost: onSessionLost,
    );
    created.add(e);
    return e;
  }
}

/// Liste de profils de la fixture, telle que l'écran des comptes la reçoit.
Map<String, dynamic> accountsData({bool empty = false}) {
  final list = Map<String, dynamic>.of(
      readFixture('profiles/list.json') as Map<String, dynamic>);
  if (empty) list['profiles'] = <dynamic>[];
  return {'phase': 'paired', ...list};
}

/// Tape [pin] sur le pavé numérique de la popup ouverte.
Future<void> tapPin(WidgetTester tester, String pin) async {
  for (final d in pin.split('')) {
    await tester.tap(
        find.descendant(of: find.byType(Dialog), matching: find.text(d)).first);
    await tester.pump();
  }
}
