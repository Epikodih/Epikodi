// Connexion : d'abord au serveur (adresse en dur, modifiable), puis à la TV
// (QR ou code à 6 chiffres).
import 'package:epikodi_remote/pairing/scan_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/harness.dart';

/// Ouvre « Changer » (adresse du serveur, en bas de l'écran).
Future<void> openServer(WidgetTester tester) async {
  await tester.tap(find.byKey(const ValueKey('change-server')));
  await settle(tester);
}

Future<void> typeServer(WidgetTester tester, String address) async {
  await tester.enterText(find.byKey(const ValueKey('address-field')), address);
  await tester.tap(button(fr.save));
  await settle(tester);
}

void main() {
  group('serveur', () {
    testWidgets('au lancement : connexion au serveur de la config',
        (tester) async {
      final world = TestWorld();
      await pumpRemote(tester, world);

      expect(world.baseUrls.first, 'http://192.168.1.42:7878');
      expect(world.backend.calls('GET', '/api/health'), hasLength(1));
      // Serveur joignable : on propose la TV (QR ou code).
      expect(find.text(fr.welcomeTitle), findsOneWidget);
      expect(button(fr.scanQr), findsOneWidget);
      expect(find.text(fr.serverLabel('192.168.1.42')), findsOneWidget);
      expect(find.text(fr.changeServer), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('serveur choisi avant : il remplace celui de la config',
        (tester) async {
      final world = TestWorld(server: '10.0.0.7:9000');
      await pumpRemote(tester, world);
      expect(world.baseUrls.first, 'http://10.0.0.7:9000');
      expect(find.text(fr.serverLabel('10.0.0.7:9000')), findsOneWidget);

      await typeCode(tester, '482916');
      expect(world.baseUrls.last, 'http://10.0.0.7:9000');
      await unmount(tester);
    });

    testWidgets('serveur injoignable : Gloubi inquiet, puis réessayer',
        (tester) async {
      final world = TestWorld()..offline = true;
      await pumpRemote(tester, world, reducedMotion: false);

      expect(find.text(fr.serverOfflineTitle), findsOneWidget);
      expect(find.text(fr.serverOfflineBody('192.168.1.42')), findsOneWidget);
      expect(gloubiShowing('worried.webp'), findsOneWidget);
      expect(button(fr.scanQr), findsNothing, reason: 'pas de TV sans serveur');

      world.offline = false;
      await tester.tap(button(fr.retry));
      await settle(tester);
      expect(find.text(fr.welcomeTitle), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('serveur injoignable : « Changer l\'adresse »', (tester) async {
      final world = TestWorld()..offline = true;
      await pumpRemote(tester, world);

      await tester.tap(button(fr.changeAddress));
      await settle(tester);
      world.offline = false;
      await typeServer(tester, '10.0.2.2');

      expect(world.store.server, '10.0.2.2:7878');
      expect(world.baseUrls.last, 'http://10.0.2.2:7878');
      expect(find.text(fr.welcomeTitle), findsOneWidget);
      expect(find.text(fr.serverLabel('10.0.2.2')), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('« Changer » : adresse pré-remplie, gardée, utilisée',
        (tester) async {
      final world = TestWorld();
      await pumpRemote(tester, world);
      await openServer(tester);

      final field = tester
          .widget<TextField>(find.byKey(const ValueKey('address-field')));
      expect(field.controller!.text, '192.168.1.42');

      await typeServer(tester, '10.0.2.2:9000');
      expect(world.store.server, '10.0.2.2:9000');
      expect(find.text(fr.serverLabel('10.0.2.2:9000')), findsOneWidget);

      await typeCode(tester, '482916');
      expect(world.baseUrls.last, 'http://10.0.2.2:9000');
      expect(world.store.saved!.host, '10.0.2.2');
      expect(world.store.saved!.port, 9000);
      expect(find.byKey(const ValueKey('key-ok')), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('adresse invalide : message, on reste sur la saisie',
        (tester) async {
      final world = TestWorld();
      await pumpRemote(tester, world);
      await openServer(tester);

      await typeServer(tester, 'pas une adresse');
      expect(find.text(fr.addressInvalid), findsOneWidget);
      expect(world.store.server, isNull);
      expect(find.byKey(const ValueKey('address-field')), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('retour sans enregistrer : même serveur', (tester) async {
      final world = TestWorld();
      await pumpRemote(tester, world);
      await openServer(tester);
      await tester.enterText(
          find.byKey(const ValueKey('address-field')), '10.9.9.9');

      await tester.tap(find.byIcon(Icons.arrow_back_rounded));
      await settle(tester);
      expect(find.text(fr.serverLabel('192.168.1.42')), findsOneWidget);
      expect(find.text(fr.welcomeTitle), findsOneWidget);
      expect(world.store.server, isNull);
      await unmount(tester);
    });

    testWidgets('« Adresse par défaut » revient au serveur de la config',
        (tester) async {
      final world = TestWorld(server: '10.0.0.7:9000');
      await pumpRemote(tester, world);
      await openServer(tester);

      await tester.tap(button(fr.resetServer('192.168.1.42')));
      await settle(tester);
      expect(world.store.server, isNull);
      expect(find.text(fr.serverLabel('192.168.1.42')), findsOneWidget);
      expect(world.baseUrls.last, 'http://192.168.1.42:7878');
      await unmount(tester);
    });

    testWidgets('serveur de la config : pas de bouton « par défaut »',
        (tester) async {
      await pumpRemote(tester, TestWorld());
      await openServer(tester);
      expect(button(fr.resetServer('192.168.1.42')), findsNothing);
      await unmount(tester);
    });
  });

  group('Gloubi', () {
    testWidgets('sort du sol et dit bonjour en arrivant', (tester) async {
      await pumpRemote(tester, TestWorld(), reducedMotion: false);
      expect(gloubiShowing('arrival.webp'), findsOneWidget);

      await tester.pump(const Duration(milliseconds: 1400));
      expect(gloubiShowing('wave.webp'), findsOneWidget);

      await tester.pump(const Duration(milliseconds: 1700));
      expect(gloubiShowing('idle.webp'), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('animations réduites : image fixe', (tester) async {
      await pumpRemote(tester, TestWorld());
      // Le logo du haut + la grande mascotte, toutes deux fixes.
      expect(gloubiShowing('mascot.png'), findsNWidgets(2));
      expect(gloubiShowing('arrival.webp'), findsNothing);
      await unmount(tester);
    });
  });

  group('TV : code', () {
    testWidgets('code complet : appairage', (tester) async {
      final world = TestWorld();
      await pumpRemote(tester, world);

      await typeCode(tester, '48291');
      expect(world.backend.calls('POST', '/api/remote/pair'), isEmpty);
      expect(isEnabled(tester, fr.connect), isFalse);

      await typeCode(tester, '482916'); // 6 chiffres : envoi automatique
      final sent = world.backend.last('POST', '/api/remote/pair')!;
      expect(sent.json, readFixture('pairing/request.json'),
          reason: 'code, nom, install_id et plateforme comme la fixture');

      // Token gardé pour les prochains lancements.
      final saved = world.store.saved!;
      expect(saved.token, 'mc_9fK2xQ7LrT3vB8nW5yZ1aD6hJ4sE0pUc');
      expect(saved.host, '192.168.1.42');
      expect(saved.port, 7878);
      expect(saved.expiresAt, DateTime.utc(2026, 12, 7, 18, 31, 12));

      // Écran « Connecté » : la session est lue avec le nouveau token.
      expect(find.byKey(const ValueKey('key-ok')), findsOneWidget);
      expect(button(fr.profileMenuSemantics('Léa')), findsOneWidget,
          reason: 'profil actif sur le bouton');
      expect(
          world.backend.last('GET', '/api/session')!.headers['Authorization'],
          'Bearer ${saved.token}');
      await unmount(tester);
    });

    testWidgets('le clavier n\'accepte que des chiffres', (tester) async {
      final world = TestWorld();
      await pumpRemote(tester, world);

      await typeCode(tester, '12ab34');
      expect(find.text('1'), findsOneWidget);
      expect(find.text('a'), findsNothing);
      expect(world.backend.calls('POST', '/api/remote/pair'), isEmpty);
      await unmount(tester);
    });

    testWidgets('code incorrect : essais restants, code effacé, Gloubi inquiet',
        (tester) async {
      final world = TestWorld();
      world.backend.fail('POST', '/api/remote/pair',
          status: 403,
          code: 'pairing_code_invalid',
          details: {'attempts_left': 2});
      await pumpRemote(tester, world, reducedMotion: false);

      await typeCode(tester, '000000');
      expect(find.text(fr.errorCodeInvalid(2)), findsOneWidget);
      expect(fr.errorCodeInvalid(2), contains('2 essais'));
      expect(gloubiShowing('worried.webp'), findsOneWidget);
      expect(find.text('0'), findsNothing, reason: 'code effacé');
      expect(world.store.saved, isNull);

      world.backend.reset('POST', '/api/remote/pair');
      await typeCode(tester, '482916');
      expect(find.byKey(const ValueKey('key-ok')), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('code expiré', (tester) async {
      final world = TestWorld();
      world.backend.fail('POST', '/api/remote/pair',
          status: 410, code: 'pairing_code_expired');
      await pumpRemote(tester, world);

      await typeCode(tester, '482916');
      expect(find.text(fr.errorCodeExpired), findsOneWidget);
      expect(gloubiShowing('worried.png'), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('trop d\'essais : bloqué pendant retry_after_s', (tester) async {
      final world = TestWorld();
      world.backend.fail('POST', '/api/remote/pair',
          status: 429,
          code: 'pairing_too_many_attempts',
          details: {'retry_after_s': 3});
      await pumpRemote(tester, world);

      await typeCode(tester, '482916');
      expect(find.text(fr.errorTooManyAttempts(3)), findsOneWidget);

      world.backend.reset('POST', '/api/remote/pair');
      await typeCode(tester, '482916');
      expect(world.backend.calls('POST', '/api/remote/pair'), hasLength(1),
          reason: 'pas de nouvel essai pendant le blocage');
      expect(isEnabled(tester, fr.connect), isFalse);

      await tester.pump(const Duration(seconds: 1));
      expect(find.text(fr.errorTooManyAttempts(2)), findsOneWidget);

      await tester.pump(const Duration(seconds: 3));
      expect(find.textContaining('Trop d\'essais'), findsNothing);
      await typeCode(tester, '482916');
      expect(world.backend.calls('POST', '/api/remote/pair'), hasLength(2));
      expect(find.byKey(const ValueKey('key-ok')), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('serveur perdu pendant l\'appairage : message avec l\'adresse',
        (tester) async {
      final world = TestWorld();
      await pumpRemote(tester, world, reducedMotion: false);
      world.offline = true;

      await typeCode(tester, '482916');
      expect(find.text(fr.errorUnreachable('192.168.1.42')), findsOneWidget);
      expect(gloubiShowing('worried.webp'), findsOneWidget);
      await unmount(tester);
    });
  });

  group('TV : QR', () {
    testWidgets('QR de la TV (fixtures) : appairé sans rien taper',
        (tester) async {
      final world = TestWorld(scanResult: fixtureQr);
      await pumpRemote(tester, world);

      await tester.tap(button(fr.scanQr));
      await settle(tester);
      expect(world.scans, 1);
      final sent = world.backend.last('POST', '/api/remote/pair')!;
      expect(sent.json, readFixture('pairing/request.json'),
          reason: 'même requête que le code tapé');
      expect(find.byKey(const ValueKey('key-ok')), findsOneWidget);
      // Le serveur reste celui auquel on est connecté.
      expect(world.baseUrls.toSet(), {'http://192.168.1.42:7878'});
      await unmount(tester);
    });

    testWidgets('QR sans code (ancien serveur) : on tape ensuite le code',
        (tester) async {
      final world = TestWorld(
          scanResult: 'mediacenter://pair?host=192.168.1.42&port=7878&sid=a3f9c2e1');
      await pumpRemote(tester, world);

      await tester.tap(button(fr.scanQr));
      await settle(tester);
      expect(world.scans, 1);
      expect(find.text(fr.qrNoCode), findsOneWidget);
      expect(world.backend.calls('POST', '/api/remote/pair'), isEmpty);

      await typeCode(tester, '482916');
      expect(find.byKey(const ValueKey('key-ok')), findsOneWidget);
      // Le serveur reste celui auquel on est connecté.
      expect(world.store.saved!.host, '192.168.1.42');
      await unmount(tester);
    });

    testWidgets('QR avec code : appairage direct', (tester) async {
      final world = TestWorld(
          scanResult:
              'mediacenter://pair?host=192.168.1.42&port=7878&sid=a3f9c2e1&code=482916');
      await pumpRemote(tester, world);

      await tester.tap(button(fr.scanQr));
      await settle(tester);
      expect(world.backend.last('POST', '/api/remote/pair')!.json['code'],
          '482916');
      expect(find.byKey(const ValueKey('key-ok')), findsOneWidget);
      await unmount(tester);
    });

    testWidgets('QR qui ne vient pas d\'une TV : message, Gloubi inquiet',
        (tester) async {
      final world = TestWorld(scanResult: 'https://example.com');
      await pumpRemote(tester, world, reducedMotion: false);

      await tester.tap(button(fr.scanQr));
      await settle(tester);
      expect(find.text(fr.scanNotTv), findsOneWidget);
      expect(gloubiShowing('worried.webp'), findsOneWidget);
      expect(world.backend.calls('POST', '/api/remote/pair'), isEmpty);
      await unmount(tester);
    });

    testWidgets('scan annulé ou « Taper le code » : rien ne change',
        (tester) async {
      final world = TestWorld(scanResult: null);
      await pumpRemote(tester, world);
      await tester.tap(button(fr.scanQr));
      await settle(tester);
      expect(find.text(fr.welcomeTitle), findsOneWidget);
      expect(find.text(fr.scanNotTv), findsNothing);

      world.scanResult = ScanScreen.manualEntry;
      await tester.tap(button(fr.scanQr));
      await settle(tester);
      expect(find.text(fr.welcomeTitle), findsOneWidget);
      expect(find.text(fr.scanNotTv), findsNothing);
      await unmount(tester);
    });
  });
}
