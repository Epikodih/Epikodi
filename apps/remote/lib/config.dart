/// Réglages de la télécommande.
library;

const appVersion = '0.1.0';

/// Adresse du serveur API du media center, utilisée tant que l'utilisateur
/// n'en a pas choisi une autre (bouton « Changer » sous le code).
/// Format : `ip` (port 7878) ou `ip:port`.
/// Dans l'émulateur Android, le PC est `10.0.2.2`.
const defaultServerAddress = '192.168.1.31:7878';

/// Plateforme envoyée au serveur à l'appairage (`device.platform`).
const devicePlatform = 'android';

/// Nom utilisé si le modèle du téléphone est inconnu.
const fallbackDeviceName = 'Android';

/// Largeur maximale du contenu (tablettes, téléphone en paysage).
const maxContentWidth = 480.0;
