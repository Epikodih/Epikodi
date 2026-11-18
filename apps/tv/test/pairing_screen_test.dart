import 'package:epikodi_tv/accounts/accounts_screen.dart';
import 'package:epikodi_tv/pairing/pairing_panel.dart';
import 'package:epikodi_tv/pairing/pairing_screen.dart';
import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/harness.dart';

void main() {
  late FixtureBackend backend;
  late FakeEventsHub events;

  setUp(() {
    backend = FixtureBackend();
    events = FakeEventsHub();
  });

  Future<void> pumpPairing(WidgetTester tester) async {
    await pumpTv(
      tester,
      PairingScreen(
        createClient: () => fixtureClient(backend),
        createEvents: events.create,
      ),
    );
    await settle(tester);
  }

  Finder codeShown(String code) =>
      semanticsLabel(fr.pairingCodeSemantics(code.split('').join(' ')));

  Finder qrWith(String payload) => find.byWidgetPredicate(
        (w) =>
            w.key is ValueKey<String> &&
            (w.key! as ValueKey<String>).value.startsWith('$payload@'),
        description: 'QR($payload)',
      );

  group('démarrage', () {
    testWidgets('affiche un chargement puis le code et le QR de connect',
        (tester) async {
      await pumpTv(
        tester,
        PairingScreen(
            createClient: () => fixtureClient(backend),
            createEvents: events.create),
      );
      expect(find.byType(CircularProgressIndicator), findsOneWidget);

      await settle(tester);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(find.byType(PairingPanel), findsOneWidget);
      expect(codeShown('482916'), findsOneWidget);
      expect(
          qrWith('mediacenter://pair?host=192.168.1.42&port=7878&sid=a3f9c2e1'),
          findsOneWidget);
      expect(find.text(fr.expiresIn('05:00')), findsOneWidget);
      expect(find.text(fr.pairingTitle), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('connect envoie modèle, plateforme et version', (tester) async {
      await pumpPairing(tester);
      final sent = backend.last('POST', '/api/tv/connect')!.json;
      expect(sent.keys, unorderedEquals(['model', 'platform', 'app_version']));
      expect(sent['platform'], anyOf('windows', 'linux', 'macos'));
      expect(sent['app_version'], matches(RegExp(r'^\d+\.\d+\.\d+$')));
      await unmount(tester);
    });

    testWidgets('serveur injoignable : erreur puis « Réessayer » reconnecte',
        (tester) async {
      backend.fail('POST', '/api/tv/connect',
          status: 503, code: 'starting', message: 'Serveur en démarrage.');
      await pumpPairing(tester);

      expect(find.text('Serveur en démarrage.'), findsOneWidget);
      expect(gloubiShowing('worried.png'), findsOneWidget,
          reason: 'Gloubi inquiet');
      expect(find.byType(PairingPanel), findsNothing);

      backend.reset('POST', '/api/tv/connect');
      await tester.tap(find.text(fr.retry));
      await settle(tester);

      expect(codeShown('482916'), findsOneWidget);
      expect(backend.calls('POST', '/api/tv/connect'), hasLength(2));
      await unmount(tester);
    });

    testWidgets('réponse inattendue : message d\'erreur générique',
        (tester) async {
      backend.on('POST', '/api/tv/connect', (_) => (201, {'oups': true}));
      await pumpPairing(tester);
      expect(
          find.textContaining(fr.unexpectedError('').trim()), findsOneWidget);
      await unmount(tester);
    });
  });

  group('Gloubi', () {
    Future<void> pumpAnimated(WidgetTester tester) async {
      await pumpTv(
        tester,
        PairingScreen(
            createClient: () => fixtureClient(backend),
            createEvents: events.create),
        reducedMotion: false,
      );
    }

    testWidgets(
        'démarrage : il sort du sol, dit bonjour, puis le code apparaît',
        (tester) async {
      await pumpAnimated(tester);
      expect(gloubiShowing('arrival.webp'), findsOneWidget);

      // Le serveur a répondu, mais on laisse Gloubi finir son bonjour.
      await settle(tester);
      expect(codeShown('482916'), findsNothing);
      await tester.pump(GloubiAnimation.arrival.duration);
      expect(gloubiShowing('wave.webp'), findsOneWidget);

      await tester.pump(Gloubi.greetingDuration);
      await settle(tester);
      expect(codeShown('482916'), findsOneWidget);
      // Le compte à rebours part de la réponse du serveur, pas de la fin
      // du bonjour : quelques secondes sont déjà passées.
      expect(find.textContaining(fr.expiresIn('04:5')), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('« Nouveau code » : pas de nouveau bonjour', (tester) async {
      await pumpAnimated(tester);
      await tester.pump(Gloubi.greetingDuration);
      await settle(tester);

      await tester.tap(find.text(fr.newCode));
      await settle(tester);
      expect(codeShown('735104'), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('serveur injoignable : Gloubi inquiet, tout de suite',
        (tester) async {
      backend.fail('POST', '/api/tv/connect',
          status: 503, code: 'starting', message: 'Serveur en démarrage.');
      await pumpAnimated(tester);
      await settle(tester);
      expect(gloubiShowing('worried.webp'), findsOneWidget);
      expect(find.text('Serveur en démarrage.'), findsOneWidget);
      await unmount(tester);
    });

    testWidgets("animations réduites : images fixes, pas d'attente",
        (tester) async {
      await pumpTv(
        tester,
        PairingScreen(
            createClient: () => fixtureClient(backend),
            createEvents: events.create),
      );
      expect(gloubiShowing('mascot.png'), findsOneWidget);
      await settle(tester);
      expect(codeShown('482916'), findsOneWidget);
      await unmount(tester);
    });
  });

  group('compte à rebours', () {
    testWidgets('décompte chaque seconde', (tester) async {
      await pumpPairing(tester);
      await tester.pump(const Duration(seconds: 1));
      expect(find.text(fr.expiresIn('04:59')), findsOneWidget);

      await tester.pump(const Duration(seconds: 59));
      expect(find.text(fr.expiresIn('04:00')), findsOneWidget);
      await unmount(tester);
    });

    testWidgets(
        'code expiré : demande un nouveau code via refresh (pas connect)',
        (tester) async {
      await pumpPairing(tester);
      await tester.pump(const Duration(seconds: 300));
      await settle(tester);

      expect(backend.calls('POST', '/api/tv/pairing/refresh'), hasLength(1));
      expect(backend.last('POST', '/api/tv/pairing/refresh')!.json,
          {'tv_session_id': 'a3f9c2e1'});
      expect(backend.calls('POST', '/api/tv/connect'), hasLength(1));
      expect(codeShown('735104'), findsOneWidget);
      expect(find.text(fr.expiresIn('05:00')), findsOneWidget);
      await unmount(tester);
    });
  });

  group('« Nouveau code »', () {
    testWidgets('demande un code puis bloque le bouton 5 s', (tester) async {
      await pumpPairing(tester);
      await tester.tap(find.text(fr.newCode));
      await settle(tester);

      expect(codeShown('735104'), findsOneWidget);
      expect(backend.calls('POST', '/api/tv/pairing/refresh'), hasLength(1));
      expect(find.text(fr.newCodeCooldown(5)), findsOneWidget);

      // Pendant le délai, un nouvel appui ne fait rien.
      await tester.tap(find.text(fr.newCodeCooldown(5)));
      await settle(tester);
      expect(backend.calls('POST', '/api/tv/pairing/refresh'), hasLength(1));

      await tester.pump(const Duration(seconds: 5));
      expect(find.text(fr.newCode), findsOneWidget);
      await tester.tap(find.text(fr.newCode));
      await settle(tester);
      expect(backend.calls('POST', '/api/tv/pairing/refresh'), hasLength(2));
      await unmount(tester);
    });

    testWidgets('Entrée sur le bouton (télécommande) demande aussi un code',
        (tester) async {
      await pumpPairing(tester);
      // Le bouton a le focus au démarrage (autofocus).
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await settle(tester);
      expect(backend.calls('POST', '/api/tv/pairing/refresh'), hasLength(1));
      await unmount(tester);
    });

    testWidgets('session oubliée par le serveur : repli sur connect',
        (tester) async {
      await pumpPairing(tester);
      backend.fail('POST', '/api/tv/pairing/refresh',
          status: 404, code: 'tv_session_not_found');
      await tester.tap(find.text(fr.newCode));
      await settle(tester);

      expect(backend.calls('POST', '/api/tv/connect'), hasLength(2));
      expect(codeShown('482916'), findsOneWidget);
      expect(gloubiShowing('worried.png'), findsNothing);
      await unmount(tester);
    });

    testWidgets('autre erreur de refresh : affichée, pas de repli',
        (tester) async {
      await pumpPairing(tester);
      backend.fail('POST', '/api/tv/pairing/refresh',
          status: 500, code: 'internal', message: 'Boum.');
      await tester.tap(find.text(fr.newCode));
      await settle(tester);

      expect(find.text('Boum.'), findsOneWidget);
      expect(backend.calls('POST', '/api/tv/connect'), hasLength(1));
      await unmount(tester);
    });
  });

  group('flux temps réel', () {
    testWidgets('s\'abonne une seule fois avec la session de connect',
        (tester) async {
      await pumpPairing(tester);
      expect(events.created, hasLength(1));
      expect(events.last.sessionId, 'a3f9c2e1');
      expect(events.last.started, isTrue);

      await tester.tap(find.text(fr.newCode));
      await settle(tester);
      expect(events.created, hasLength(1),
          reason: 'même session : pas de 2e abonnement');
      await unmount(tester);
    });

    testWidgets('phase « pairing » : reste sur l\'écran d\'appairage',
        (tester) async {
      await pumpPairing(tester);
      events.last.emit({'phase': 'pairing'});
      await settle(tester);
      expect(find.byType(PairingPanel), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('phase « paired » : passe à « Qui regarde ? » avec les comptes',
        (tester) async {
      await pumpPairing(tester);
      final first = events.last;
      first.emit(accountsData());
      await settle(tester);
      await tester.pump(const Duration(seconds: 1));

      expect(find.byType(PairingPanel), findsNothing);
      expect(find.byType(AccountsScreen), findsOneWidget);
      expect(find.text(fr.whoIsWatching), findsOneWidget);
      expect(find.text('Marc'), findsOneWidget);
      expect(find.text('Léa'), findsOneWidget);
      expect(first.closed, isTrue,
          reason: 'le flux est fermé après la passation');
      await unmount(tester);
    });

    testWidgets(
        'serveur redémarré (session perdue) : nouveau connect et réabonnement',
        (tester) async {
      await pumpPairing(tester);
      backend.on('POST', '/api/tv/connect', (_) {
        final json =
            Map<String, dynamic>.of(readFixture('tv/connect_response.json'));
        json['tv_session_id'] = 'nouvelle';
        return (201, json);
      });
      events.last.loseSession();
      await settle(tester);

      expect(backend.calls('POST', '/api/tv/connect'), hasLength(2));
      expect(events.created, hasLength(2));
      expect(events.last.sessionId, 'nouvelle');
      await unmount(tester);
    });

    testWidgets(
        '« Appairage » depuis les comptes : retour à un nouvel écran d\'appairage',
        (tester) async {
      await pumpPairing(tester);
      events.last.emit(accountsData());
      await settle(tester);
      await tester.pump(const Duration(seconds: 1));

      await tester.tap(find.text(fr.pairingButton));
      await settle(tester);
      await tester.pump(const Duration(seconds: 1));

      expect(find.byType(PairingPanel), findsOneWidget);
      expect(backend.calls('POST', '/api/tv/connect'), hasLength(2),
          reason: 'nouveau connect');
      await unmount(tester);
    });
  });

  testWidgets('fenêtre minimale 960×540 : pas de débordement', (tester) async {
    await pumpTv(
      tester,
      PairingScreen(
          createClient: () => fixtureClient(backend),
          createEvents: events.create),
      size: const Size(960, 540),
    );
    await settle(tester);
    expect(tester.takeException(), isNull);
    expect(codeShown('482916'), findsOneWidget);
    await unmount(tester);
  });
}
