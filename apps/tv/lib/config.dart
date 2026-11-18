import 'dart:io' show Platform;

const appVersion = '0.1.0';

/// Adresse du serveur (le backend de test écoute sur le port 7878).
const serverUrl = 'http://127.0.0.1:7878';

// ---- Réglages d'affichage de l'app (modifier ici, puis hot restart : R) ----

/// Taille de tous les textes (1.0 = normal, de 0.8 à 2.0).
const defaultTextScale = 1.0;

/// Taille des QR codes (1.0 = normal, de 0.6 à 1.6).
const defaultQrScale = 1.2;

/// Désactive les animations.
const defaultReducedMotion = false;

// ---- Fenêtre ----

enum WindowMode {
  /// Plein écran sans bordure (démo / vraie TV).
  fullscreen,

  /// Fenêtre agrandie : remplit l'écran mais garde la barre de titre
  /// (réduire, agrandir, fermer). Pratique pour développer.
  maximized,

  /// Fenêtre normale (voir windowedWidth x windowedHeight).
  windowed,
}

/// Mode au lancement. Passe à WindowMode.fullscreen pour la démo.
const windowMode = WindowMode.maximized;

/// Taille de la fenêtre en mode [WindowMode.windowed], et taille de départ
/// avant d'être agrandie/plein écran (le système s'en sert pour « restaurer »).
const windowedWidth = 1280.0;
const windowedHeight = 720.0;

/// Taille minimale quand on redimensionne la fenêtre à la main : en dessous,
/// l'écran d'appairage devient trop petit pour rester lisible.
const minWindowWidth = 960.0;
const minWindowHeight = 540.0;

/// Pause entre « fenêtre prête » et son affichage (après avoir appliqué
/// taille + agrandissement/plein écran). Évite l'image affichée en double au
/// lancement sous Windows : la surface a le temps de se redimensionner avant
/// la première image visible. Mettre Duration.zero pour désactiver.
const windowShowDelay = Duration(milliseconds: 150);

/// Échap quitte le plein écran. À passer à false quand « Retour » (Échap)
/// servira à naviguer entre les écrans.
const escapeLeavesFullScreen = true;

const newCodeCooldown = Duration(seconds: 5);

/// "windows", "linux" ou "macos".
String get tvPlatform => Platform.operatingSystem;

String get deviceModel => 'Desktop ${Platform.operatingSystem}';
