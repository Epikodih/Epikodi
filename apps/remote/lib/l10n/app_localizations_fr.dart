// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get appTitle => 'GloubiTV Remote';

  @override
  String get welcomeTitle => 'Connecte-toi à ta TV';

  @override
  String get welcomeBody =>
      'Scanne le QR code affiché sur la TV, ou tape le code à 6 chiffres.';

  @override
  String get scanQr => 'Scanner le QR code de la TV';

  @override
  String get addressHint => '192.168.1.42';

  @override
  String get addressInvalid =>
      'Adresse invalide. Exemple : 192.168.1.42 ou 192.168.1.42:7878';

  @override
  String get scanTitle => 'Vise le QR code de la TV';

  @override
  String get scanNotTv => 'Ce QR code ne vient pas d\'une TV GloubiTV.';

  @override
  String get scanNoPermission =>
      'GloubiTV a besoin de la caméra pour lire le QR code. Autorise-la dans les réglages, ou tape le code affiché sur la TV.';

  @override
  String get scanUnavailable =>
      'La caméra n\'est pas disponible. Tape le code affiché sur la TV.';

  @override
  String get codeSemantics => 'Code d\'appairage à 6 chiffres';

  @override
  String get connect => 'Connecter';

  @override
  String get changeTv => 'Changer de TV';

  @override
  String get pairing => 'Connexion à la TV…';

  @override
  String errorCodeInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count essais',
      one: '1 essai',
    );
    return 'Code incorrect. Encore $_temp0.';
  }

  @override
  String get errorCodeInvalidNoCount =>
      'Code incorrect. Vérifie le code affiché sur la TV.';

  @override
  String get errorCodeExpired =>
      'Ce code a expiré. Tape le nouveau code affiché sur la TV.';

  @override
  String errorTooManyAttempts(int seconds) {
    return 'Trop d\'essais. Réessaie dans $seconds s.';
  }

  @override
  String get errorTooManyAttemptsNoDelay =>
      'Trop d\'essais. Attends un peu, puis tape le nouveau code de la TV.';

  @override
  String errorUnreachable(String address) {
    return 'Impossible de joindre la TV à l\'adresse $address. Vérifie que le téléphone et la TV sont sur le même Wi-Fi.';
  }

  @override
  String errorUnexpected(String message) {
    return 'Une erreur est survenue : $message';
  }

  @override
  String get retry => 'Réessayer';

  @override
  String get checking => 'Connexion à ta TV…';

  @override
  String get guestProfile => 'Invité';

  @override
  String get cancel => 'Annuler';

  @override
  String get sessionExpired =>
      'La TV ne reconnaît plus ce téléphone. Connecte-toi de nouveau.';

  @override
  String get tvUnreachableTitle => 'Ta TV ne répond pas';

  @override
  String tvUnreachableBody(String address) {
    return 'Vérifie qu\'elle est allumée et sur le même Wi-Fi que ce téléphone ($address).';
  }

  @override
  String get gloubiLabel => 'Gloubi, la mascotte';

  @override
  String serverLabel(String address) {
    return 'Serveur : $address';
  }

  @override
  String get changeServer => 'Changer';

  @override
  String changeServerSemantics(String address) {
    return 'Changer l\'adresse du serveur, actuellement $address';
  }

  @override
  String get serverTitle => 'Adresse du serveur';

  @override
  String get serverBody =>
      'L\'adresse du serveur du media center sur ton réseau.';

  @override
  String get save => 'Enregistrer';

  @override
  String resetServer(String address) {
    return 'Adresse par défaut ($address)';
  }

  @override
  String get orTypeCode => 'ou tape le code';

  @override
  String get qrNoCode =>
      'QR code lu. Tape maintenant le code à 6 chiffres affiché sous le QR.';

  @override
  String get connectingServer => 'Connexion au serveur…';

  @override
  String get serverOfflineTitle => 'Serveur injoignable';

  @override
  String serverOfflineBody(String address) {
    return 'Impossible de joindre le serveur à l\'adresse $address. Vérifie l\'adresse et que le téléphone est sur le même Wi-Fi.';
  }

  @override
  String get changeAddress => 'Changer l\'adresse';

  @override
  String get typeCodeInstead => 'Taper le code';

  @override
  String get profile => 'Profil';

  @override
  String get switchProfile => 'Changer de profil';

  @override
  String get pairingMenu => 'Appairage';

  @override
  String profileMenuSemantics(String name) {
    return 'Menu du profil $name';
  }

  @override
  String get closeMenu => 'Fermer le menu';

  @override
  String get noProfile => 'Choisir un profil';

  @override
  String get keyUp => 'Haut';

  @override
  String get keyDown => 'Bas';

  @override
  String get keyLeft => 'Gauche';

  @override
  String get keyRight => 'Droite';

  @override
  String get keyOk => 'OK';

  @override
  String get keyBack => 'Retour';

  @override
  String get keyHome => 'Accueil';

  @override
  String get whoIsWatching => 'Qui regarde ?';

  @override
  String get currentProfile => 'Profil actuel';

  @override
  String get profileLocked => 'Protégé par un code PIN';

  @override
  String pinTitle(String name) {
    return 'Code PIN de $name';
  }

  @override
  String get pinSemantics => 'Code PIN à 4 chiffres';

  @override
  String pinInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count essais',
      one: '1 essai',
    );
    return 'PIN incorrect. Encore $_temp0.';
  }

  @override
  String get pinInvalidNoCount => 'PIN incorrect.';

  @override
  String pinLocked(int seconds) {
    return 'Trop d\'essais. Réessaie dans $seconds s.';
  }

  @override
  String get pinLockedNoDelay => 'Trop d\'essais. Réessaie plus tard.';

  @override
  String get profilesError => 'Impossible de charger les comptes.';

  @override
  String get pairingConfirmTitle => 'Revenir à l\'appairage ?';

  @override
  String get pairingConfirmBody =>
      'La TV revient à son écran d\'appairage et ce téléphone s\'en déconnecte. Il faudra de nouveau scanner le QR code ou taper le code.';

  @override
  String get confirm => 'Continuer';

  @override
  String get infoMenu => 'Infos';

  @override
  String get infoTitle => 'Infos';

  @override
  String get infoServer => 'Serveur';

  @override
  String get infoPhone => 'Ce téléphone';

  @override
  String get infoProfile => 'Profil actif';

  @override
  String get infoVersion => 'Version de l\'app';

  @override
  String get changeServerConfirmTitle => 'Changer de serveur ?';

  @override
  String get changeServerConfirmBody =>
      'Ce téléphone sera déconnecté de la TV actuelle, puis se connectera au nouveau serveur.';

  @override
  String get keyboardTitle => 'Écrire sur la TV';

  @override
  String get keyboardDone => 'Valider';

  @override
  String get keyboardClose => 'Fermer le clavier';

  @override
  String languageToggle(String language) {
    return 'Langue : passer la TV et le téléphone en $language';
  }
}
