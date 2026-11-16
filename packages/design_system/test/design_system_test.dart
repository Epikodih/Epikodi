import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

Widget wrap(Widget child, {UiSettings? settings}) => MaterialApp(
      theme: AppTheme.dark(),
      builder: (context, c) =>
          AppUiScope(settings: settings ?? UiSettings(), child: c!),
      home: Scaffold(body: Center(child: child)),
    );

void main() {
  group('UiSettings', () {
    test('valeurs bornées (texte 0.8–2.0, QR 0.6–1.6)', () {
      final s = UiSettings(textScale: 5, qrScale: 0.1);
      expect(s.textScale, 2.0);
      expect(s.qrScale, 0.6);
      s
        ..textScale = 0.1
        ..qrScale = 9;
      expect(s.textScale, 0.8);
      expect(s.qrScale, 1.6);
    });

    test('paliers de texte : monte et descend sans dépasser les bornes', () {
      final s = UiSettings();
      s.increaseText();
      expect(s.textScale, 1.25);
      for (var i = 0; i < 10; i++) {
        s.increaseText();
      }
      expect(s.textScale, UiSettings.textScaleSteps.last);
      for (var i = 0; i < 10; i++) {
        s.decreaseText();
      }
      expect(s.textScale, UiSettings.textScaleSteps.first);
    });

    test('prévient les écouteurs à chaque changement', () {
      final s = UiSettings();
      var n = 0;
      s.addListener(() => n++);
      s
        ..textScale = 1.5
        ..qrScale = 1.2
        ..reducedMotion = true;
      expect(n, 3);
    });
  });

  group('AppUiScope', () {
    testWidgets('applique la taille du texte et « animations réduites »',
        (tester) async {
      final settings = UiSettings(textScale: 1.5, reducedMotion: true);
      late MediaQueryData mq;
      await tester.pumpWidget(wrap(Builder(builder: (c) {
        mq = MediaQuery.of(c);
        return const SizedBox();
      }), settings: settings));
      expect(mq.textScaler.scale(10), 15);
      expect(mq.disableAnimations, isTrue);

      settings.textScale = 2.0;
      await tester.pump();
      expect(mq.textScaler.scale(10), 20);
    });
  });

  group('FocusableCard', () {
    testWidgets('clic, Entrée, Select et bouton A déclenchent onTap',
        (tester) async {
      var taps = 0;
      await tester.pumpWidget(wrap(FocusableCard(
        autofocus: true,
        onTap: () => taps++,
        child: const Text('carte'),
      )));
      await tester.pump();

      await tester.tap(find.text('carte'));
      for (final k in [
        LogicalKeyboardKey.enter,
        LogicalKeyboardKey.numpadEnter,
        LogicalKeyboardKey.select,
        LogicalKeyboardKey.gameButtonA,
      ]) {
        await tester.sendKeyEvent(k);
      }
      expect(taps, 5);
    });

    testWidgets('les autres touches sont laissées aux parents', (tester) async {
      var taps = 0;
      await tester.pumpWidget(wrap(FocusableCard(
        autofocus: true,
        onTap: () => taps++,
        child: const Text('carte'),
      )));
      await tester.pump();
      final handled = await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
      expect(handled, isFalse);
      expect(taps, 0);
    });

    testWidgets('les flèches déplacent le focus d\'une carte à l\'autre',
        (tester) async {
      final tapped = <String>[];
      await tester
          .pumpWidget(wrap(Row(mainAxisSize: MainAxisSize.min, children: [
        for (final n in ['A', 'B', 'C'])
          Padding(
            padding: const EdgeInsets.all(8),
            child: FocusableCard(
                autofocus: n == 'A',
                onTap: () => tapped.add(n),
                child: Text(n)),
          ),
      ])));
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      expect(tapped, ['B', 'C']);
    });

    testWidgets('libellé et rôle de bouton pour le lecteur d\'écran',
        (tester) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(wrap(FocusableCard(
        onTap: () {},
        semanticLabel: 'Nouveau code',
        child: const Icon(Icons.refresh),
      )));
      expect(
        tester.getSemantics(find.byType(FocusableCard)),
        matchesSemantics(
          label: 'Nouveau code',
          isButton: true,
          isFocusable: true,
          hasTapAction: true,
        ),
      );
      handle.dispose();
    });

    testWidgets('sans onTap : pas un bouton', (tester) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(
          wrap(const FocusableCard(semanticLabel: 'x', child: Text('x'))));
      expect(
          tester
              .getSemantics(find.byType(FocusableCard))
              .flagsCollection
              .isButton,
          isFalse);
      handle.dispose();
    });
  });
}
