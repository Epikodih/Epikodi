import 'package:flutter/material.dart';
import 'package:media_kit/media_kit.dart';

import 'app.dart';
import 'window.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await setupWindow();

  // Charge libmpv une fois pour toute l'app (moteur de lecture, ADR 0001).
  MediaKit.ensureInitialized();
  runApp(const GloubiTvApp());
}
