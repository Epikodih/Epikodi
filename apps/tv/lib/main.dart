import 'package:flutter/material.dart';

import 'app.dart';
import 'window.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await setupWindow();

  // Plus tard, pour la lecture vidéo : MediaKit.ensureInitialized();
  runApp(const GloubiTvApp());
}
