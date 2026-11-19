// Touches de la télécommande -> focus de la TV (comme le clavier).
import 'package:design_system/design_system.dart';
import 'package:epikodi_tv/accounts/create_account_screen.dart';
import 'package:epikodi_tv/api/api.dart';
import 'package:epikodi_tv/locale.dart';
import 'package:epikodi_tv/remote/remote_keys.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/harness.dart';

void main() {
  late GlobalKey<NavigatorState> nav;
  late RemoteKeyHandler remote;
  late List<String> tapped;

  /// Trois cartes en ligne, puis une quatrième en dessous.
  Future<void> pump(WidgetTester tester) async {
    nav = GlobalKey<NavigatorState>();
    remote = RemoteKeyHandler(nav);
    tapped = [];
    Widget card(String name, {bool autofocus = false}) => FocusableCard(
          autofocus: autofocus,
          onTap: () => tapped.add(name),
          semanticLabel: name,
          child: SizedBox(width: 100, height: 60, child: Text(name)),
        );
    await tester.pumpWidget(MaterialApp(
      navigatorKey: nav,
      home: Scaffold(
        body: Column(children: [
          Row(children: [card('A', autofocus: true), card('B'), card('C')]),
          const SizedBox(height: 40),
          card('D'),
        ]),
      ),
    ));
    await tester.pump();
  }

  String focused() {
    final ctx = FocusManager.instance.primaryFocus!.context!;
    return (ctx.findAncestorWidgetOfExactType<FocusableCard>()!.semanticLabel)!;
  }

  Future<void> press(WidgetTester tester, String key) async {
    remote.press(key);
    await tester.pumpAndSettle();
  }

  testWidgets('flèches : le focus se déplace', (tester) async {
    await pump(tester);
    expect(focused(), 'A');
    await press(tester, 'right');
    expect(focused(), 'B');
    await press(tester, 'right');
    expect(focused(), 'C');
    await press(tester, 'left');
    expect(focused(), 'B');
    await press(tester, 'down');
    expect(focused(), 'D');
    await press(tester, 'up');
    expect(focused(), isNot('D'));
  });

  testWidgets('rien dans cette direction : ordre de tabulation',
      (tester) async {
    await pump(tester);
    await press(tester, 'right');
    await press(tester, 'right'); // C, dernier de la ligne
    await press(tester, 'right'); // rien à droite -> suivant (D)
    expect(focused(), 'D');
  });

  testWidgets('OK active l\'élément qui a le focus', (tester) async {
    await pump(tester);
    await press(tester, 'right');
    await press(tester, 'ok');
    expect(tapped, ['B']);
  });

  testWidgets('événement du serveur {"key": ...}', (tester) async {
    await pump(tester);
    remote.handle({'key': 'right'});
    await tester.pumpAndSettle();
    expect(focused(), 'B');
    remote.handle({'shortcut': 'profile'}); // ignoré pour l'instant
    remote.handle({'key': 42});
    await tester.pumpAndSettle();
    expect(focused(), 'B');
  });

  testWidgets('Retour : écran précédent ; Accueil : premier écran',
      (tester) async {
    await pump(tester);
    nav.currentState!.push(MaterialPageRoute<void>(
        builder: (_) => const Scaffold(body: Text('page 2'))));
    await tester.pumpAndSettle();
    nav.currentState!.push(MaterialPageRoute<void>(
        builder: (_) => const Scaffold(body: Text('page 3'))));
    await tester.pumpAndSettle();

    await press(tester, 'back');
    expect(find.text('page 2'), findsOneWidget);

    await press(tester, 'home');
    expect(find.text('A'), findsOneWidget);
    expect(find.text('page 2'), findsNothing);
  });

  group('création de compte (champ texte)', () {
    late FixtureBackend backend;
    late List<TextFieldInfo?> announced;
    late TextFieldWatcher watcher;

    Future<void> pumpCreate(WidgetTester tester) async {
      backend = FixtureBackend();
      announced = [];
      watcher = TextFieldWatcher(announced.add);
      addTearDown(watcher.dispose);
      nav = GlobalKey<NavigatorState>();
      remote = RemoteKeyHandler(nav, onTextFieldOk: watcher.resend);
      await pumpTv(
          tester, CreateAccountScreen(api: Api(fixtureClient(backend).http)));
      await tester.pump();
    }

    Future<void> press(WidgetTester tester, String key) async {
      remote.press(key);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
    }

    testWidgets('le champ « Nom » a le focus et est annoncé au téléphone',
        (tester) async {
      await pumpCreate(tester);
      expect(focusedEditable(), isNotNull);
      final field = announced.whereType<TextFieldInfo>().last;
      expect(field.label, fr.name);
      expect(field.value, '');
      expect(field.maxLength, 20);
      expect(field.number, isFalse);
      expect(field.toJson()['active'], isTrue);
    });

    testWidgets('les flèches sortent du champ texte et y reviennent',
        (tester) async {
      await pumpCreate(tester);
      await press(tester, 'down');
      expect(focusedEditable(), isNull, reason: 'sorti du champ');
      expect(announced.last, isNull, reason: 'le téléphone ferme son clavier');

      // Tous les éléments de la page sont atteignables aux flèches
      // (exploration depuis le champ Nom, dans les 4 directions). Par nom :
      // certains boutons sont reconstruits (nouveau FocusNode) en route.
      String name(FocusNode n) =>
          n.context
              ?.findAncestorWidgetOfExactType<FocusableCard>()
              ?.semanticLabel ??
          (n.context?.findAncestorWidgetOfExactType<TextField>() != null
              ? 'champ'
              : describeIdentity(n));
      Iterable<FocusNode> nodes() => FocusManager.instance.primaryFocus!
          .nearestScope!.traversalDescendants
          .where((n) => n.canRequestFocus && !n.skipTraversal);
      /// Place le focus sur [node] et oublie l'historique des flèches (sinon
      /// la flèche opposée à la précédente « revient en arrière »).
      Future<void> focusOn(WidgetTester tester, FocusNode node) async {
        node.requestFocus();
        await tester.pump();
        FocusTraversalGroup.maybeOf(node.context!)
            ?.invalidateScopeData(node.nearestScope!);
      }

      final all = nodes().map(name).toSet();
      final reached = <String>{};
      final todo = [name(FocusManager.instance.primaryFocus!)];
      while (todo.isNotEmpty) {
        final from = todo.removeLast();
        if (!reached.add(from)) continue;
        for (final dir in ['up', 'down', 'left', 'right']) {
          await focusOn(tester, nodes().firstWhere((n) => name(n) == from));
          await press(tester, dir);
          todo.add(name(FocusManager.instance.primaryFocus!));
        }
      }
      expect(all.difference(reached), isEmpty,
          reason: 'éléments impossibles à atteindre aux flèches');
      expect(reached.length, greaterThan(5));

      // Depuis « Créer le compte », on remonte jusqu'au champ Nom.
      await focusOn(tester,
          nodes().firstWhere((n) => name(n) == fr.createAccountButton));
      for (var i = 0; i < 20 && focusedEditable() == null; i++) {
        await press(tester, 'up');
      }
      expect(focusedEditable(), isNotNull, reason: 'retour au champ Nom');
    });

    testWidgets('texte tapé sur le téléphone : écrit dans le champ',
        (tester) async {
      await pumpCreate(tester);
      remote.handle({'text': 'Léa', 'done': false});
      await tester.pump();
      expect(find.text('Léa'), findsOneWidget);
      expect(focusedEditable(), isNotNull, reason: 'pas encore validé');

      // Longueur max du champ respectée.
      remote.typeText('x' * 40);
      await tester.pump();
      expect(focusedEditable()!.widget.controller.text.length, 20);
    });

    testWidgets('texte validé : le focus passe à la suite', (tester) async {
      await pumpCreate(tester);
      remote.handle({'text': 'Tom', 'done': true});
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('Tom'), findsOneWidget);
      expect(focusedEditable(), isNull);
      // L'élément juste en dessous du champ, pas le haut de la page.
      final card = FocusManager.instance.primaryFocus!.context!
          .findAncestorWidgetOfExactType<FocusableCard>();
      expect(card?.semanticLabel, fr.decreaseAge);
    });

    testWidgets('valider deux fois de suite ne ramène jamais en haut',
        (tester) async {
      await pumpCreate(tester);
      remote.typeText('Tom', done: true);
      await tester.pump();
      await press(tester, 'up'); // retour sur le champ
      expect(focusedEditable(), isNotNull);
      remote.typeText('Tomy', done: true);
      await tester.pump();
      final card = FocusManager.instance.primaryFocus!.context!
          .findAncestorWidgetOfExactType<FocusableCard>();
      expect(card?.semanticLabel, fr.decreaseAge);
      expect(find.text('Tomy'), findsOneWidget);
    });

    testWidgets('OK sur le champ : clavier redemandé au téléphone',
        (tester) async {
      await pumpCreate(tester);
      final before = announced.length;
      await press(tester, 'ok');
      expect(announced.length, before + 1);
      expect(announced.last!.label, fr.name);
    });
  });

  test('raccourci langue : la TV passe en anglais puis en français', () {
    final before = appLocale.value;
    addTearDown(() => appLocale.value = before);
    final handler = RemoteKeyHandler(GlobalKey<NavigatorState>());

    handler.handle({'shortcut': 'language', 'language': 'en'});
    expect(appLocale.value, const Locale('en'));
    handler.handle({'shortcut': 'language', 'language': 'fr'});
    expect(appLocale.value, const Locale('fr'));

    // Langue inconnue ou absente : rien ne change.
    handler.handle({'shortcut': 'language', 'language': 'de'});
    handler.handle({'shortcut': 'language'});
    expect(appLocale.value, const Locale('fr'));
  });
}
