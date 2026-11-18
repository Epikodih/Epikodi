import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_fr.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
      : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations? of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations);
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
    delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
  ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('fr')
  ];

  /// No description provided for @connectingToServer.
  ///
  /// In fr, this message translates to:
  /// **'Connexion au serveur'**
  String get connectingToServer;

  /// No description provided for @retry.
  ///
  /// In fr, this message translates to:
  /// **'Réessayer'**
  String get retry;

  /// No description provided for @retryConnectionLabel.
  ///
  /// In fr, this message translates to:
  /// **'Réessayer de se connecter au serveur'**
  String get retryConnectionLabel;

  /// No description provided for @unexpectedError.
  ///
  /// In fr, this message translates to:
  /// **'Erreur inattendue : {details}'**
  String unexpectedError(String details);

  /// No description provided for @createAccount.
  ///
  /// In fr, this message translates to:
  /// **'Créer un compte'**
  String get createAccount;

  /// No description provided for @createAccountButton.
  ///
  /// In fr, this message translates to:
  /// **'Créer le compte'**
  String get createAccountButton;

  /// No description provided for @creating.
  ///
  /// In fr, this message translates to:
  /// **'Création...'**
  String get creating;

  /// No description provided for @back.
  ///
  /// In fr, this message translates to:
  /// **'Retour'**
  String get back;

  /// No description provided for @name.
  ///
  /// In fr, this message translates to:
  /// **'Nom'**
  String get name;

  /// No description provided for @nameHint.
  ///
  /// In fr, this message translates to:
  /// **'Prénom ou pseudo'**
  String get nameHint;

  /// No description provided for @enterName.
  ///
  /// In fr, this message translates to:
  /// **'Entrez un nom.'**
  String get enterName;

  /// No description provided for @choosePhoto.
  ///
  /// In fr, this message translates to:
  /// **'Choisir ma photo'**
  String get choosePhoto;

  /// No description provided for @imagePickError.
  ///
  /// In fr, this message translates to:
  /// **'Impossible de choisir une image ici.'**
  String get imagePickError;

  /// No description provided for @avatarIcon.
  ///
  /// In fr, this message translates to:
  /// **'Icône {number}'**
  String avatarIcon(int number);

  /// No description provided for @avatarIconSelected.
  ///
  /// In fr, this message translates to:
  /// **'Icône {number}, sélectionnée'**
  String avatarIconSelected(int number);

  /// No description provided for @age.
  ///
  /// In fr, this message translates to:
  /// **'Âge'**
  String get age;

  /// No description provided for @ageYears.
  ///
  /// In fr, this message translates to:
  /// **'{age} ans'**
  String ageYears(int age);

  /// No description provided for @decreaseAge.
  ///
  /// In fr, this message translates to:
  /// **'Diminuer l\'âge'**
  String get decreaseAge;

  /// No description provided for @increaseAge.
  ///
  /// In fr, this message translates to:
  /// **'Augmenter l\'âge'**
  String get increaseAge;

  /// No description provided for @language.
  ///
  /// In fr, this message translates to:
  /// **'Langue'**
  String get language;

  /// No description provided for @childProfile.
  ///
  /// In fr, this message translates to:
  /// **'Profil enfant'**
  String get childProfile;

  /// No description provided for @childProfileSemantic.
  ///
  /// In fr, this message translates to:
  /// **'Profil enfant (contrôle parental)'**
  String get childProfileSemantic;

  /// No description provided for @yes.
  ///
  /// In fr, this message translates to:
  /// **'Oui'**
  String get yes;

  /// No description provided for @no.
  ///
  /// In fr, this message translates to:
  /// **'Non'**
  String get no;

  /// No description provided for @noContentBlocked.
  ///
  /// In fr, this message translates to:
  /// **'Aucun contenu bloqué'**
  String get noContentBlocked;

  /// No description provided for @noContentBlockedAdult.
  ///
  /// In fr, this message translates to:
  /// **'Aucun contenu bloqué (18 ans et +)'**
  String get noContentBlockedAdult;

  /// No description provided for @blocksRatingAndAbove.
  ///
  /// In fr, this message translates to:
  /// **'Bloque les +{rating} et plus'**
  String blocksRatingAndAbove(int rating);

  /// No description provided for @pairingTitle.
  ///
  /// In fr, this message translates to:
  /// **'Connecter la télécommande'**
  String get pairingTitle;

  /// No description provided for @pairingInstruction.
  ///
  /// In fr, this message translates to:
  /// **'Scannez le QR code ou saisissez le code dans l’application.'**
  String get pairingInstruction;

  /// No description provided for @newCode.
  ///
  /// In fr, this message translates to:
  /// **'Nouveau code'**
  String get newCode;

  /// No description provided for @newCodeCooldown.
  ///
  /// In fr, this message translates to:
  /// **'Nouveau code ({seconds} s)'**
  String newCodeCooldown(int seconds);

  /// No description provided for @newCodeSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Générer un nouveau code et un nouveau QR code'**
  String get newCodeSemantics;

  /// No description provided for @newCodeCoolingSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Nouveau code, disponible dans quelques secondes'**
  String get newCodeCoolingSemantics;

  /// No description provided for @expiresIn.
  ///
  /// In fr, this message translates to:
  /// **'Expire dans {time}'**
  String expiresIn(String time);

  /// No description provided for @expiresSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Le code expire dans {minutes} minutes'**
  String expiresSemantics(int minutes);

  /// No description provided for @pairingCodeSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Code d’appairage : {code}'**
  String pairingCodeSemantics(String code);

  /// No description provided for @pairingQrSemantics.
  ///
  /// In fr, this message translates to:
  /// **'QR code d’appairage'**
  String get pairingQrSemantics;

  /// No description provided for @switchLanguageLabel.
  ///
  /// In fr, this message translates to:
  /// **'Changer de langue'**
  String get switchLanguageLabel;

  /// No description provided for @noAccountYet.
  ///
  /// In fr, this message translates to:
  /// **'Aucun compte pour l’instant'**
  String get noAccountYet;

  /// No description provided for @whoIsWatching.
  ///
  /// In fr, this message translates to:
  /// **'Qui regarde ?'**
  String get whoIsWatching;

  /// No description provided for @createOrGuest.
  ///
  /// In fr, this message translates to:
  /// **'Créez un compte ou regardez en tant qu’invité'**
  String get createOrGuest;

  /// No description provided for @createCard.
  ///
  /// In fr, this message translates to:
  /// **'Créer'**
  String get createCard;

  /// No description provided for @guestCard.
  ///
  /// In fr, this message translates to:
  /// **'Invité'**
  String get guestCard;

  /// No description provided for @continueAsGuest.
  ///
  /// In fr, this message translates to:
  /// **'Continuer en invité'**
  String get continueAsGuest;

  /// No description provided for @accountSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Compte {name}'**
  String accountSemantics(String name);

  /// No description provided for @accountLockedSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Compte {name}, protégé par un code PIN'**
  String accountLockedSemantics(String name);

  /// No description provided for @pairingButton.
  ///
  /// In fr, this message translates to:
  /// **'Appairage'**
  String get pairingButton;

  /// No description provided for @backToPairingLabel.
  ///
  /// In fr, this message translates to:
  /// **'Revenir à l’écran d’appairage'**
  String get backToPairingLabel;

  /// No description provided for @profile.
  ///
  /// In fr, this message translates to:
  /// **'Profil'**
  String get profile;

  /// No description provided for @profileMenuSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Menu du profil {name}'**
  String profileMenuSemantics(String name);

  /// No description provided for @closeMenu.
  ///
  /// In fr, this message translates to:
  /// **'Fermer le menu'**
  String get closeMenu;

  /// No description provided for @switchProfile.
  ///
  /// In fr, this message translates to:
  /// **'Changer de profil'**
  String get switchProfile;

  /// No description provided for @pinTitle.
  ///
  /// In fr, this message translates to:
  /// **'Code PIN requis'**
  String get pinTitle;

  /// No description provided for @pinSubtitle.
  ///
  /// In fr, this message translates to:
  /// **'Entrez le code PIN pour modifier la restriction d’âge.'**
  String get pinSubtitle;

  /// No description provided for @pinWrong.
  ///
  /// In fr, this message translates to:
  /// **'Code PIN incorrect.'**
  String get pinWrong;

  /// No description provided for @pinDigitSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Chiffre {digit}'**
  String pinDigitSemantics(String digit);

  /// No description provided for @deleteDigit.
  ///
  /// In fr, this message translates to:
  /// **'Effacer'**
  String get deleteDigit;

  /// No description provided for @cancel.
  ///
  /// In fr, this message translates to:
  /// **'Annuler'**
  String get cancel;

  /// No description provided for @save.
  ///
  /// In fr, this message translates to:
  /// **'Enregistrer'**
  String get save;

  /// No description provided for @saving.
  ///
  /// In fr, this message translates to:
  /// **'Enregistrement...'**
  String get saving;

  /// No description provided for @pinWillBeAsked.
  ///
  /// In fr, this message translates to:
  /// **'Le code PIN sera demandé à l’enregistrement.'**
  String get pinWillBeAsked;

  /// No description provided for @sectionMovies.
  ///
  /// In fr, this message translates to:
  /// **'Films'**
  String get sectionMovies;

  /// No description provided for @sectionSeries.
  ///
  /// In fr, this message translates to:
  /// **'Séries'**
  String get sectionSeries;

  /// No description provided for @sectionMusic.
  ///
  /// In fr, this message translates to:
  /// **'Musique'**
  String get sectionMusic;

  /// No description provided for @sectionPodcasts.
  ///
  /// In fr, this message translates to:
  /// **'Podcasts'**
  String get sectionPodcasts;

  /// No description provided for @libraryLoadError.
  ///
  /// In fr, this message translates to:
  /// **'Impossible de charger la médiathèque.'**
  String get libraryLoadError;

  /// No description provided for @noMediaForProfile.
  ///
  /// In fr, this message translates to:
  /// **'Aucun média disponible pour ce profil.'**
  String get noMediaForProfile;

  /// No description provided for @guestName.
  ///
  /// In fr, this message translates to:
  /// **'Invité'**
  String get guestName;

  /// No description provided for @playerPlay.
  ///
  /// In fr, this message translates to:
  /// **'Lecture'**
  String get playerPlay;

  /// No description provided for @playerPause.
  ///
  /// In fr, this message translates to:
  /// **'Pause'**
  String get playerPause;

  /// No description provided for @playerRewind.
  ///
  /// In fr, this message translates to:
  /// **'Reculer de 10 secondes'**
  String get playerRewind;

  /// No description provided for @playerForward.
  ///
  /// In fr, this message translates to:
  /// **'Avancer de 30 secondes'**
  String get playerForward;

  /// No description provided for @playerLoading.
  ///
  /// In fr, this message translates to:
  /// **'Chargement…'**
  String get playerLoading;

  /// No description provided for @playerPaused.
  ///
  /// In fr, this message translates to:
  /// **'En pause'**
  String get playerPaused;

  /// No description provided for @playerEnded.
  ///
  /// In fr, this message translates to:
  /// **'Lecture terminée'**
  String get playerEnded;

  /// No description provided for @playerPositionLabel.
  ///
  /// In fr, this message translates to:
  /// **'Position de lecture'**
  String get playerPositionLabel;

  /// No description provided for @playerPosition.
  ///
  /// In fr, this message translates to:
  /// **'{position} sur {duration}'**
  String playerPosition(String position, String duration);

  /// No description provided for @playerError.
  ///
  /// In fr, this message translates to:
  /// **'Lecture impossible : {details}'**
  String playerError(String details);

  /// No description provided for @playerTracks.
  ///
  /// In fr, this message translates to:
  /// **'Audio et sous-titres'**
  String get playerTracks;

  /// No description provided for @playerAudio.
  ///
  /// In fr, this message translates to:
  /// **'Audio'**
  String get playerAudio;

  /// No description provided for @playerSubtitles.
  ///
  /// In fr, this message translates to:
  /// **'Sous-titres'**
  String get playerSubtitles;

  /// No description provided for @playerSubtitlesOff.
  ///
  /// In fr, this message translates to:
  /// **'Désactivés'**
  String get playerSubtitlesOff;

  /// No description provided for @playerAudioDescription.
  ///
  /// In fr, this message translates to:
  /// **'Audiodescription'**
  String get playerAudioDescription;

  /// No description provided for @playerTrackNumber.
  ///
  /// In fr, this message translates to:
  /// **'Piste {number}'**
  String playerTrackNumber(int number);

  /// No description provided for @playerNoTrack.
  ///
  /// In fr, this message translates to:
  /// **'Aucune piste dans ce fichier'**
  String get playerNoTrack;

  /// No description provided for @subtitleSize.
  ///
  /// In fr, this message translates to:
  /// **'Taille des sous-titres'**
  String get subtitleSize;

  /// No description provided for @subtitleSizeSmall.
  ///
  /// In fr, this message translates to:
  /// **'Petite'**
  String get subtitleSizeSmall;

  /// No description provided for @subtitleSizeNormal.
  ///
  /// In fr, this message translates to:
  /// **'Normale'**
  String get subtitleSizeNormal;

  /// No description provided for @subtitleSizeLarge.
  ///
  /// In fr, this message translates to:
  /// **'Grande'**
  String get subtitleSizeLarge;

  /// No description provided for @subtitleSizeExtraLarge.
  ///
  /// In fr, this message translates to:
  /// **'Très grande'**
  String get subtitleSizeExtraLarge;

  /// No description provided for @subtitleColor.
  ///
  /// In fr, this message translates to:
  /// **'Couleur du texte'**
  String get subtitleColor;

  /// No description provided for @subtitleColorWhite.
  ///
  /// In fr, this message translates to:
  /// **'Blanc'**
  String get subtitleColorWhite;

  /// No description provided for @subtitleColorYellow.
  ///
  /// In fr, this message translates to:
  /// **'Jaune'**
  String get subtitleColorYellow;

  /// No description provided for @subtitleColorCyan.
  ///
  /// In fr, this message translates to:
  /// **'Cyan'**
  String get subtitleColorCyan;

  /// No description provided for @subtitleBackground.
  ///
  /// In fr, this message translates to:
  /// **'Fond'**
  String get subtitleBackground;

  /// No description provided for @subtitleBackgroundNone.
  ///
  /// In fr, this message translates to:
  /// **'Aucun'**
  String get subtitleBackgroundNone;

  /// No description provided for @subtitleBackgroundTranslucent.
  ///
  /// In fr, this message translates to:
  /// **'Semi-transparent'**
  String get subtitleBackgroundTranslucent;

  /// No description provided for @subtitleBackgroundOpaque.
  ///
  /// In fr, this message translates to:
  /// **'Noir'**
  String get subtitleBackgroundOpaque;

  /// No description provided for @subtitlePreview.
  ///
  /// In fr, this message translates to:
  /// **'Aperçu des sous-titres'**
  String get subtitlePreview;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'fr'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'fr':
      return AppLocalizationsFr();
  }

  throw FlutterError(
      'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
      'an issue with the localizations generation tool. Please file an issue '
      'on GitHub with a reproducible sample app and the gen-l10n configuration '
      'that was used.');
}
