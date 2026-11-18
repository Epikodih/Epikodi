// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get connectingToServer => 'Connexion au serveur';

  @override
  String get retry => 'Réessayer';

  @override
  String get retryConnectionLabel => 'Réessayer de se connecter au serveur';

  @override
  String unexpectedError(String details) {
    return 'Erreur inattendue : $details';
  }

  @override
  String get createAccount => 'Créer un compte';

  @override
  String get createAccountButton => 'Créer le compte';

  @override
  String get creating => 'Création...';

  @override
  String get back => 'Retour';

  @override
  String get name => 'Nom';

  @override
  String get nameHint => 'Prénom ou pseudo';

  @override
  String get enterName => 'Entrez un nom.';

  @override
  String get choosePhoto => 'Choisir ma photo';

  @override
  String get imagePickError => 'Impossible de choisir une image ici.';

  @override
  String avatarIcon(int number) {
    return 'Icône $number';
  }

  @override
  String avatarIconSelected(int number) {
    return 'Icône $number, sélectionnée';
  }

  @override
  String get age => 'Âge';

  @override
  String ageYears(int age) {
    return '$age ans';
  }

  @override
  String get decreaseAge => 'Diminuer l\'âge';

  @override
  String get increaseAge => 'Augmenter l\'âge';

  @override
  String get language => 'Langue';

  @override
  String get childProfile => 'Profil enfant';

  @override
  String get childProfileSemantic => 'Profil enfant (contrôle parental)';

  @override
  String get yes => 'Oui';

  @override
  String get no => 'Non';

  @override
  String get noContentBlocked => 'Aucun contenu bloqué';

  @override
  String get noContentBlockedAdult => 'Aucun contenu bloqué (18 ans et +)';

  @override
  String blocksRatingAndAbove(int rating) {
    return 'Bloque les +$rating et plus';
  }

  @override
  String get pairingTitle => 'Connecter la télécommande';

  @override
  String get pairingInstruction =>
      'Scannez le QR code ou saisissez le code dans l’application.';

  @override
  String get newCode => 'Nouveau code';

  @override
  String newCodeCooldown(int seconds) {
    return 'Nouveau code ($seconds s)';
  }

  @override
  String get newCodeSemantics =>
      'Générer un nouveau code et un nouveau QR code';

  @override
  String get newCodeCoolingSemantics =>
      'Nouveau code, disponible dans quelques secondes';

  @override
  String expiresIn(String time) {
    return 'Expire dans $time';
  }

  @override
  String expiresSemantics(int minutes) {
    return 'Le code expire dans $minutes minutes';
  }

  @override
  String pairingCodeSemantics(String code) {
    return 'Code d’appairage : $code';
  }

  @override
  String get pairingQrSemantics => 'QR code d’appairage';

  @override
  String get switchLanguageLabel => 'Changer de langue';

  @override
  String get noAccountYet => 'Aucun compte pour l’instant';

  @override
  String get whoIsWatching => 'Qui regarde ?';

  @override
  String get createOrGuest => 'Créez un compte ou regardez en tant qu’invité';

  @override
  String get createCard => 'Créer';

  @override
  String get guestCard => 'Invité';

  @override
  String get continueAsGuest => 'Continuer en invité';

  @override
  String accountSemantics(String name) {
    return 'Compte $name';
  }

  @override
  String accountLockedSemantics(String name) {
    return 'Compte $name, protégé par un code PIN';
  }

  @override
  String get pairingButton => 'Appairage';

  @override
  String get backToPairingLabel => 'Revenir à l’écran d’appairage';

  @override
  String get profile => 'Profil';

  @override
  String profileMenuSemantics(String name) {
    return 'Menu du profil $name';
  }

  @override
  String get closeMenu => 'Fermer le menu';

  @override
  String get switchProfile => 'Changer de profil';

  @override
  String get pinTitle => 'Code PIN requis';

  @override
  String get pinSubtitle =>
      'Entrez le code PIN pour modifier la restriction d’âge.';

  @override
  String get pinWrong => 'Code PIN incorrect.';

  @override
  String pinDigitSemantics(String digit) {
    return 'Chiffre $digit';
  }

  @override
  String get deleteDigit => 'Effacer';

  @override
  String get cancel => 'Annuler';

  @override
  String get save => 'Enregistrer';

  @override
  String get saving => 'Enregistrement...';

  @override
  String get pinWillBeAsked => 'Le code PIN sera demandé à l’enregistrement.';

  @override
  String get sectionMovies => 'Films';

  @override
  String get sectionSeries => 'Séries';

  @override
  String get sectionMusic => 'Musique';

  @override
  String get sectionPodcasts => 'Podcasts';

  @override
  String get libraryLoadError => 'Impossible de charger la médiathèque.';

  @override
  String get noMediaForProfile => 'Aucun média disponible pour ce profil.';

  @override
  String get guestName => 'Invité';

  @override
  String get playerPlay => 'Lecture';

  @override
  String get playerPause => 'Pause';

  @override
  String get playerRewind => 'Reculer de 10 secondes';

  @override
  String get playerForward => 'Avancer de 30 secondes';

  @override
  String get playerLoading => 'Chargement…';

  @override
  String get playerPaused => 'En pause';

  @override
  String get playerEnded => 'Lecture terminée';

  @override
  String get playerPositionLabel => 'Position de lecture';

  @override
  String playerPosition(String position, String duration) {
    return '$position sur $duration';
  }

  @override
  String playerError(String details) {
    return 'Lecture impossible : $details';
  }
}
