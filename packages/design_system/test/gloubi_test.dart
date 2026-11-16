import 'package:design_system/design_system.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

AssetImage imageOf(WidgetTester tester) =>
    tester.widget<Image>(find.byType(Image)).image as AssetImage;

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('Gloubi assets', () {
    // Fails if a file is missing or assets/mascot/ is not declared in pubspec.
    for (final name in [
      'mascot.png',
      ...GloubiAnimation.values.map((a) => '${a.name}.webp'),
    ]) {
      test('$name is bundled', () async {
        final data =
            await rootBundle.load('packages/design_system/assets/mascot/$name');
        expect(data.lengthInBytes, greaterThan(0));
      });
    }
  });

  group('Gloubi widget', () {
    testWidgets('shows the still image by default', (tester) async {
      await tester.pumpWidget(const Gloubi());
      expect(imageOf(tester).assetName, 'assets/mascot/mascot.png');
      expect(imageOf(tester).package, 'design_system');
    });

    testWidgets('shows the requested animation', (tester) async {
      await tester.pumpWidget(const Gloubi(animation: GloubiAnimation.wave));
      expect(imageOf(tester).assetName, 'assets/mascot/wave.webp');
    });

    testWidgets('entrance plays arrival then switches to idle', (tester) async {
      await tester.pumpWidget(const Gloubi.entrance());
      expect(imageOf(tester).assetName, 'assets/mascot/arrival.webp');

      await tester.pump(const Duration(milliseconds: 1400));
      expect(imageOf(tester).assetName, 'assets/mascot/idle.webp');
    });
  });
}
