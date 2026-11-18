import 'dart:ui' show Size;

import 'package:window_manager/window_manager.dart';

import 'config.dart';

/// Prépare la fenêtre avant le premier affichage : taille, taille minimale,
/// mode (plein écran / agrandi / fenêtré), puis seulement show().
/// La première image visible est ainsi déjà à la bonne taille.
Future<void> setupWindow() async {
  await windowManager.ensureInitialized();

  const options = WindowOptions(
    size: Size(windowedWidth, windowedHeight),
    minimumSize: Size(minWindowWidth, minWindowHeight),
    center: true,
    title: 'GloubiTV',
  );

  await windowManager.waitUntilReadyToShow(options, () async {
    switch (windowMode) {
      case WindowMode.fullscreen:
        await windowManager.setFullScreen(true);
      case WindowMode.maximized:
        await windowManager.maximize();
      case WindowMode.windowed:
        break;
    }
    // Laisse la surface se redimensionner avant la première image visible.
    await Future<void>.delayed(windowShowDelay);
    await windowManager.show();
    await windowManager.focus();
  });
}

/// F11 : bascule plein écran / fenêtre.
Future<void> toggleFullScreen() async {
  final full = await windowManager.isFullScreen();
  await windowManager.setFullScreen(!full);
}

/// Échap : quitte le plein écran (sans effet si on n'y est pas).
Future<void> leaveFullScreen() async {
  if (await windowManager.isFullScreen()) {
    await windowManager.setFullScreen(false);
  }
}
