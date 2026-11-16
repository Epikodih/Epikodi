import 'package:epikodi_tv/accounts/accounts_flow.dart';
import 'package:epikodi_tv/accounts/accounts_screen.dart';
import 'package:epikodi_tv/accounts/create_account_screen.dart';
import 'package:epikodi_tv/accounts/pin_dialog.dart';
import 'package:epikodi_tv/api/api.dart';
import 'package:epikodi_tv/library/library_screen.dart';
import 'package:epikodi_tv/widgets/profile_menu.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/harness.dart';

/// Mêmes clés et mêmes types que la fixture (les valeurs peuvent différer).
void expectShapeOf(Map<String, dynamic> sent, String fixture,
    {Set<String> without = const {}}) {
  final expected =
      Map<String, dynamic>.of(readFixture(fixture) as Map<String, dynamic>)
        ..removeWhere((k, _) => without.contains(k));
  expect(sent.keys.toSet(), expected.keys.toSet(),
      reason: 'clés envoyées vs $fixture');
  for (final k in expected.keys) {
    expect(sent[k].runtimeType, expected[k].runtimeType,
        reason: '$k vs $fixture');
  }
}

void main() {
  group('AccountsScreen (vue seule)', () {
    Future<List<String>> pumpScreen(
        WidgetTester tester, Map<String, dynamic> data) async {
      final calls = <String>[];
      await pumpTv(
        tester,
        AccountsScreen(
          data: data,
          onBackToPairing: () => calls.add('pairing'),
          onProfile: (p) => calls.add('profile:${p['id']}'),
          onCreate: () => calls.add('create'),
          onGuest: () => calls.add('guest'),
        ),
      );
      await tester.pump();
      return calls;
    }

    testWidgets('affiche les comptes, « Créer » et « Invité »', (tester) async {
      await pumpScreen(tester, accountsData());
      expect(find.text(fr.whoIsWatching), findsOneWidget);
      expect(find.text('Marc'), findsOneWidget);
      expect(find.text('Léa'), findsOneWidget);
      expect(find.text(fr.createCard), findsOneWidget);
      expect(find.text(fr.guestCard), findsOneWidget);
    });

    testWidgets('cadenas seulement sur les comptes avec PIN', (tester) async {
      await pumpScreen(tester, accountsData());
      expect(find.byIcon(Icons.lock_rounded), findsOneWidget);
      expect(semanticsLabel(fr.accountLockedSemantics('Marc')), findsOneWidget);
      expect(semanticsLabel(fr.accountSemantics('Léa')), findsOneWidget);
    });

    testWidgets('aucun compte : texte d\'aide, Créer et Invité seulement',
        (tester) async {
      await pumpScreen(tester, accountsData(empty: true));
      expect(find.text(fr.noAccountYet), findsOneWidget);
      expect(find.text(fr.createOrGuest), findsOneWidget);
      expect(find.text(fr.whoIsWatching), findsNothing);
      expect(find.text(fr.createCard), findsOneWidget);
      expect(find.text(fr.guestCard), findsOneWidget);
    });

    testWidgets('respecte can_create et guest.available du serveur',
        (tester) async {
      final data = accountsData()
        ..['can_create'] = false
        ..['guest'] = {'available': false, 'is_restricted': true};
      await pumpScreen(tester, data);
      expect(find.text(fr.createCard), findsNothing);
      expect(find.text(fr.guestCard), findsNothing);
    });

    testWidgets('chaque carte prévient le parent', (tester) async {
      final calls = await pumpScreen(tester, accountsData());
      await tester.tap(find.text('Léa'));
      await tester.tap(find.text('Marc'));
      await tester.tap(find.text(fr.createCard));
      await tester.tap(find.text(fr.guestCard));
      await tester.tap(find.text(fr.pairingButton));
      expect(calls, ['profile:2', 'profile:1', 'create', 'guest', 'pairing']);
    });

    testWidgets('le premier compte a le focus et Entrée le choisit',
        (tester) async {
      final calls = await pumpScreen(tester, accountsData());
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      expect(calls, ['profile:1']);
    });

    testWidgets('beaucoup de comptes ou petite fenêtre : pas de débordement',
        (tester) async {
      final data = accountsData();
      data['profiles'] = [
        for (var i = 1; i <= 12; i++)
          {
            'id': i,
            'name': 'Compte au nom très long $i',
            'requires_pin': i.isEven
          },
      ];
      await pumpScreen(tester, data);
      expect(tester.takeException(), isNull);

      await pumpTv(
        tester,
        AccountsScreen(
          data: data,
          onBackToPairing: () {},
          onProfile: (_) {},
          onCreate: () {},
          onGuest: () {},
        ),
        size: const Size(960, 540),
      );
      await tester.pump();
      expect(tester.takeException(), isNull);
    });

    testWidgets('le bouton de langue passe en anglais', (tester) async {
      await pumpScreen(tester, accountsData());
      await tester.tap(find.text('EN'));
      await tester.pumpAndSettle();
      expect(find.text(en.whoIsWatching), findsOneWidget);
    });
  });

  group('PinDialog (choix d\'un compte)', () {
    Future<List<String>> openPin(
      WidgetTester tester, {
      String? Function(String pin)? check,
    }) async {
      final submitted = <String>[];
      await pumpTv(
        tester,
        Builder(
          builder: (context) => TextButton(
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => PinDialog(
                profileName: 'Marc',
                onSubmit: (pin) async {
                  submitted.add(pin);
                  return check?.call(pin);
                },
              ),
            ),
            child: const Text('ouvrir'),
          ),
        ),
      );
      await tester.tap(find.text('ouvrir'));
      await tester.pumpAndSettle();
      return submitted;
    }

    testWidgets('envoie le PIN dès le 4e chiffre et se ferme s\'il est bon',
        (tester) async {
      final submitted = await openPin(tester);
      expect(find.text('pour Marc'), findsOneWidget);
      await tapPin(tester, '123');
      expect(submitted, isEmpty);
      await tapPin(tester, '4');
      await tester.pumpAndSettle();
      expect(submitted, ['1234']);
      expect(find.byType(PinDialog), findsNothing);
    });

    testWidgets('PIN refusé : message affiché, saisie effacée, popup ouverte',
        (tester) async {
      final submitted =
          await openPin(tester, check: (_) => 'Code PIN incorrect.');
      await tapPin(tester, '9999');
      await tester.pumpAndSettle();
      expect(find.text('Code PIN incorrect.'), findsOneWidget);
      expect(find.byType(PinDialog), findsOneWidget);
      expect(semanticsLabel('0 chiffres saisis sur 4'), findsOneWidget);

      // Un nouveau chiffre efface le message.
      await tapPin(tester, '1');
      await tester.pumpAndSettle();
      expect(find.text('Code PIN incorrect.'), findsNothing);
      expect(submitted, ['9999']);
    });

    testWidgets('effacer retire le dernier chiffre', (tester) async {
      final submitted = await openPin(tester);
      await tapPin(tester, '12');
      await tester.tap(find.byIcon(Icons.backspace_outlined));
      await tester.pump();
      expect(semanticsLabel('1 chiffres saisis sur 4'), findsOneWidget);
      await tapPin(tester, '345');
      await tester.pumpAndSettle();
      expect(submitted, ['1345']);
    });

    testWidgets('clavier physique : chiffres et Retour arrière',
        (tester) async {
      final submitted = await openPin(tester);
      for (final k in [LogicalKeyboardKey.digit5, LogicalKeyboardKey.digit6]) {
        await tester.sendKeyEvent(k);
      }
      await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
      for (final k in [
        LogicalKeyboardKey.digit7,
        LogicalKeyboardKey.digit8,
        LogicalKeyboardKey.digit9
      ]) {
        await tester.sendKeyEvent(k);
      }
      await tester.pumpAndSettle();
      expect(submitted, ['5789']);
    });

    testWidgets('« Annuler » ferme sans rien envoyer', (tester) async {
      final submitted = await openPin(tester);
      await tester.tap(find.byIcon(Icons.close_rounded));
      await tester.pumpAndSettle();
      expect(find.byType(PinDialog), findsNothing);
      expect(submitted, isEmpty);
    });
  });

  group('AccountsFlow (avec le faux serveur)', () {
    late FixtureBackend backend;
    late List<String> backToPairing;

    setUp(() {
      backend = FixtureBackend();
      backToPairing = [];
    });

    Future<void> pumpFlow(WidgetTester tester, {bool empty = false}) async {
      await pumpTv(
        tester,
        AccountsFlow(
          api: Api(fixtureClient(backend).http),
          data: accountsData(empty: empty),
          onBackToPairing: () => backToPairing.add('x'),
        ),
      );
      await tester.pump();
    }

    testWidgets('compte restreint : choisi sans PIN, ouvre la médiathèque',
        (tester) async {
      await pumpFlow(tester);
      await tester.tap(find.text('Léa'));
      await settle(tester);

      final sent = backend.last('POST', '/api/session/profile')!.json;
      expect(sent, {'type': 'account', 'profile_id': 2});
      expectShapeOf(sent, 'session/select_request.json', without: {'pin'});
      expect(find.byType(LibraryScreen), findsOneWidget);
    });

    testWidgets('compte avec PIN : popup, mauvais PIN refusé, bon PIN accepté',
        (tester) async {
      backend.fail('POST', '/api/session/profile',
          status: 403,
          code: 'pin_invalid',
          message: 'Code PIN incorrect.',
          details: {'attempts_left': 2});
      await pumpFlow(tester);
      await tester.tap(find.text('Marc'));
      await tester.pumpAndSettle();
      expect(find.byType(PinDialog), findsOneWidget);

      await tapPin(tester, '0000');
      await settle(tester);
      expect(find.text('Code PIN incorrect.'), findsOneWidget);
      expect(find.byType(LibraryScreen), findsNothing);

      backend.reset('POST', '/api/session/profile');
      await tapPin(tester, '1234');
      await settle(tester);
      await tester.pump(const Duration(seconds: 1));

      final sent = backend.last('POST', '/api/session/profile')!.json;
      expect(sent, {'type': 'account', 'profile_id': 1, 'pin': '1234'});
      expectShapeOf(sent, 'session/select_request.json');
      expect(find.byType(PinDialog), findsNothing);
      expect(find.byType(LibraryScreen), findsOneWidget);
    });

    testWidgets('PIN annulé : reste sur « Qui regarde ? »', (tester) async {
      await pumpFlow(tester);
      await tester.tap(find.text('Marc'));
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.close_rounded));
      await tester.pumpAndSettle();
      expect(backend.calls('POST', '/api/session/profile'), isEmpty);
      expect(find.text(fr.whoIsWatching), findsOneWidget);
    });

    testWidgets('Invité : envoie {"type": "guest"} seulement', (tester) async {
      await pumpFlow(tester);
      await tester.tap(find.text(fr.guestCard));
      await settle(tester);
      expect(backend.last('POST', '/api/session/profile')!.json,
          {'type': 'guest'});
      expect(find.byType(LibraryScreen), findsOneWidget);
    });

    testWidgets('erreur serveur : message en bas, pas de médiathèque',
        (tester) async {
      backend.fail('POST', '/api/session/profile',
          status: 404,
          code: 'profile_not_found',
          message: 'Ce profil n\'existe pas.');
      await pumpFlow(tester);
      await tester.tap(find.text('Léa'));
      await settle(tester);
      expect(find.text('Ce profil n\'existe pas.'), findsOneWidget);
      expect(find.byType(LibraryScreen), findsNothing);
    });

    testWidgets(
        '« Changer de profil » depuis la médiathèque revient aux comptes',
        (tester) async {
      await pumpFlow(tester);
      await tester.tap(find.text('Léa'));
      await settle(tester);
      await tester.pump(const Duration(seconds: 1));

      await tester.tap(find.byType(ProfileMenuButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(fr.switchProfile));
      await tester.pumpAndSettle();
      expect(find.text(fr.whoIsWatching), findsOneWidget);
    });

    testWidgets('« Appairage » prévient le parent', (tester) async {
      await pumpFlow(tester);
      await tester.tap(find.text(fr.pairingButton));
      expect(backToPairing, hasLength(1));
    });

    testWidgets('création : POST /api/profiles, le compte apparaît et s\'ouvre',
        (tester) async {
      await pumpFlow(tester, empty: true);
      await tester.tap(find.text(fr.createCard));
      await tester.pumpAndSettle();
      expect(find.byType(CreateAccountScreen), findsOneWidget);

      await tester.enterText(find.byType(TextField), '  Zoé  ');
      await tester.tap(find.text(fr.createAccountButton));
      await settle(tester);
      await tester.pump(const Duration(seconds: 1));

      final sent = backend.last('POST', '/api/profiles')!.json;
      expect(sent['name'], 'Zoé', reason: 'espaces retirés');
      expect(sent['language'], 'fr');
      expect(sent['age'], 18);
      expect(sent['is_restricted'], isFalse);
      expectShapeOf(sent, 'profiles/create_request.json', without: {'pin'});
      expect(find.byType(LibraryScreen), findsOneWidget);

      // Retour aux comptes : le compte créé (réponse du serveur) est listé.
      await tester.tap(find.byType(ProfileMenuButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(fr.switchProfile));
      await tester.pumpAndSettle();
      expect(find.text(fr.whoIsWatching), findsOneWidget);
      expect(find.text('Marc'), findsOneWidget);
    });
  });

  group('CreateAccountScreen', () {
    late FixtureBackend backend;
    setUp(() => backend = FixtureBackend());

    Future<void> pumpCreate(WidgetTester tester) async {
      await pumpTv(
          tester, CreateAccountScreen(api: Api(fixtureClient(backend).http)));
      await tester.pump();
    }

    testWidgets('nom vide : message, aucune requête', (tester) async {
      await pumpCreate(tester);
      await tester.enterText(find.byType(TextField), '   ');
      await tester.tap(find.text(fr.createAccountButton));
      await tester.pump();
      expect(find.text(fr.enterName), findsOneWidget);
      expect(backend.calls('POST', '/api/profiles'), isEmpty);
    });

    testWidgets('âge borné entre 3 et 99, profil enfant, avatar choisi',
        (tester) async {
      await pumpCreate(tester);
      expect(find.text(fr.ageYears(18)), findsOneWidget);
      await tester.tap(find.byIcon(Icons.add_rounded));
      await tester.pump();
      expect(find.text(fr.ageYears(19)), findsOneWidget);
      for (var i = 0; i < 30; i++) {
        await tester.tap(find.byIcon(Icons.remove_rounded));
      }
      await tester.pump();
      expect(find.text(fr.ageYears(3)), findsOneWidget);

      await tester.tap(find.text(fr.no)); // profil enfant : non -> oui
      await tester.pump();
      expect(find.text(fr.yes), findsOneWidget);
      expect(find.text(fr.blocksRatingAndAbove(10)), findsOneWidget);

      await tester.tap(semanticsLabel(fr.avatarIcon(3)));
      await tester.enterText(find.byType(TextField), 'Tom');
      await tester.tap(find.text(fr.createAccountButton));
      await settle(tester);

      final sent = backend.last('POST', '/api/profiles')!.json;
      expect(sent['age'], 3);
      expect(sent['is_restricted'], isTrue);
      expect(sent['icon'], 'avatar_03');
    });

    testWidgets('erreur serveur (nom pris) : message affiché, on peut corriger',
        (tester) async {
      backend.fail('POST', '/api/profiles',
          status: 409, code: 'profile_name_taken', message: 'Nom déjà pris.');
      await pumpCreate(tester);
      await tester.enterText(find.byType(TextField), 'Marc');
      await tester.tap(find.text(fr.createAccountButton));
      await settle(tester);
      expect(find.text('Nom déjà pris.'), findsOneWidget);
      expect(find.text(fr.createAccountButton), findsOneWidget,
          reason: 'plus en « création… »');
    });

    testWidgets('fenêtre minimale : pas de débordement', (tester) async {
      await pumpTv(
          tester, CreateAccountScreen(api: Api(fixtureClient(backend).http)),
          size: const Size(960, 540));
      await tester.pump();
      expect(tester.takeException(), isNull);
    });
  });
}
