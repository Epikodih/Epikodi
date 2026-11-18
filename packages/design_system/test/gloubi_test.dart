import 'dart:ui' as ui;

import 'package:design_system/design_system.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

// In the app, MaterialApp provides Directionality; the semantic label needs it.
Widget wrap(Widget child) =>
    Directionality(textDirection: TextDirection.ltr, child: child);

AssetImage imageOf(WidgetTester tester) =>
    tester.widget<Image>(find.byType(Image)).image as AssetImage;

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('Gloubi assets', () {
    // Fails if a file is missing or assets/mascot/ is not declared in pubspec.
    for (final name in [
      'mascot.png',
      'worried.png',
      ...GloubiAnimation.values.map((a) => '${a.name}.webp'),
    ]) {
      test('$name is bundled', () async {
        final data =
            await rootBundle.load('packages/design_system/assets/mascot/$name');
        expect(data.lengthInBytes, greaterThan(0));
      });
    }
  });

  group('Gloubi animation files', () {
    // Nombre de boucles écrit dans le fichier WebP (chunk ANIM) : 0 = infini.
    Future<int> loopCount(String name) async {
      final data =
          await rootBundle.load('packages/design_system/assets/mascot/$name');
      final b = data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
      var i = 12; // après « RIFF....WEBP »
      while (i + 8 <= b.length) {
        final tag = String.fromCharCodes(b.sublist(i, i + 4));
        final size = b[i + 4] | b[i + 5] << 8 | b[i + 6] << 16 | b[i + 7] << 24;
        if (tag == 'ANIM') return b[i + 12] | b[i + 13] << 8;
        i += 8 + size + (size & 1);
      }
      throw StateError('$name : pas de chunk ANIM');
    }

    test('idle et worried bouclent', () async {
      expect(await loopCount('idle.webp'), 0);
      expect(await loopCount('worried.webp'), 0);
    });

    testWidgets("le décodeur de Flutter fait boucler worried à l'infini",
        (tester) async {
      final repeat = await tester.runAsync(() async {
        final data = await rootBundle
            .load('packages/design_system/assets/mascot/worried.webp');
        final codec = await ui.instantiateImageCodec(data.buffer.asUint8List());
        final count = codec.repetitionCount;
        codec.dispose();
        return count;
      });
      expect(repeat, -1, reason: '-1 = boucle infinie pour Flutter');
    });

    test('arrival et wave jouent une seule fois', () async {
      expect(await loopCount('arrival.webp'), 1);
      expect(await loopCount('wave.webp'), 1);
    });
  });

  group('Gloubi widget', () {
    testWidgets('shows the still image by default', (tester) async {
      await tester.pumpWidget(wrap(const Gloubi()));
      expect(imageOf(tester).assetName, 'assets/mascot/mascot.png');
      expect(imageOf(tester).package, 'design_system');
    });

    testWidgets('shows the requested animation', (tester) async {
      await tester
          .pumpWidget(wrap(const Gloubi(animation: GloubiAnimation.wave)));
      expect(imageOf(tester).assetName, 'assets/mascot/wave.webp');
    });

    testWidgets('entrance plays arrival then switches to idle', (tester) async {
      await tester.pumpWidget(wrap(const Gloubi.entrance()));
      expect(imageOf(tester).assetName, 'assets/mascot/arrival.webp');

      await tester.pump(const Duration(milliseconds: 1400));
      expect(imageOf(tester).assetName, 'assets/mascot/idle.webp');
    });
    testWidgets('hello waves then idles', (tester) async {
      await tester.pumpWidget(wrap(const Gloubi.hello()));
      expect(imageOf(tester).assetName, 'assets/mascot/wave.webp');

      await tester.pump(GloubiAnimation.wave.duration);
      expect(imageOf(tester).assetName, 'assets/mascot/idle.webp');
    });

    testWidgets('greeting: arrival, wave, then idle', (tester) async {
      await tester.pumpWidget(wrap(const Gloubi.greeting()));
      expect(imageOf(tester).assetName, 'assets/mascot/arrival.webp');

      await tester.pump(GloubiAnimation.arrival.duration);
      expect(imageOf(tester).assetName, 'assets/mascot/wave.webp');

      await tester.pump(GloubiAnimation.wave.duration);
      expect(imageOf(tester).assetName, 'assets/mascot/idle.webp');
      expect(Gloubi.greetingDuration,
          GloubiAnimation.arrival.duration + GloubiAnimation.wave.duration);
    });

    testWidgets('changing the animation restarts it', (tester) async {
      await tester.pumpWidget(wrap(const Gloubi.hello()));
      await tester
          .pumpWidget(wrap(const Gloubi(animation: GloubiAnimation.worried)));
      expect(imageOf(tester).assetName, 'assets/mascot/worried.webp');
      await tester.pump(const Duration(seconds: 5));
      expect(imageOf(tester).assetName, 'assets/mascot/worried.webp');
    });
  });

  group('Gloubi with animations disabled', () {
    Widget reduced(Widget child) => MediaQuery(
          data: const MediaQueryData(disableAnimations: true),
          child: wrap(child),
        );

    testWidgets('greeting shows the still smiling image', (tester) async {
      await tester.pumpWidget(reduced(const Gloubi.greeting()));
      expect(imageOf(tester).assetName, 'assets/mascot/mascot.png');
      await tester.pump(const Duration(seconds: 5));
      expect(imageOf(tester).assetName, 'assets/mascot/mascot.png');
    });

    testWidgets('worried shows the still worried image', (tester) async {
      await tester.pumpWidget(
          reduced(const Gloubi(animation: GloubiAnimation.worried)));
      expect(imageOf(tester).assetName, 'assets/mascot/worried.png');
    });
  });

  group('GloubiLogo', () {
    testWidgets('utilise le widget Gloubi (image fixe) + « GloubiTV »',
        (tester) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(wrap(const GloubiLogo(size: 56)));
      expect(find.byType(Gloubi), findsOneWidget);
      expect(imageOf(tester).assetName, 'assets/mascot/mascot.png');
      expect(find.bySemanticsLabel('GloubiTV'), findsOneWidget);
      handle.dispose();
    });
  });
}
