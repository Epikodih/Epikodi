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

  /// No description provided for @appTitle.
  ///
  /// In fr, this message translates to:
  /// **'GloubiTV Remote'**
  String get appTitle;

  /// No description provided for @welcomeTitle.
  ///
  /// In fr, this message translates to:
  /// **'Connecte-toi à ta TV'**
  String get welcomeTitle;

  /// No description provided for @welcomeBody.
  ///
  /// In fr, this message translates to:
  /// **'Scanne le QR code affiché sur la TV, ou tape le code à 6 chiffres.'**
  String get welcomeBody;

  /// No description provided for @scanQr.
  ///
  /// In fr, this message translates to:
  /// **'Scanner le QR code de la TV'**
  String get scanQr;

  /// No description provided for @addressHint.
  ///
  /// In fr, this message translates to:
  /// **'192.168.1.42'**
  String get addressHint;

  /// No description provided for @addressInvalid.
  ///
  /// In fr, this message translates to:
  /// **'Adresse invalide. Exemple : 192.168.1.42 ou 192.168.1.42:7878'**
  String get addressInvalid;

  /// No description provided for @scanTitle.
  ///
  /// In fr, this message translates to:
  /// **'Vise le QR code de la TV'**
  String get scanTitle;

  /// No description provided for @scanNotTv.
  ///
  /// In fr, this message translates to:
  /// **'Ce QR code ne vient pas d\'une TV GloubiTV.'**
  String get scanNotTv;

  /// No description provided for @scanNoPermission.
  ///
  /// In fr, this message translates to:
  /// **'GloubiTV a besoin de la caméra pour lire le QR code. Autorise-la dans les réglages, ou tape le code affiché sur la TV.'**
  String get scanNoPermission;

  /// No description provided for @scanUnavailable.
  ///
  /// In fr, this message translates to:
  /// **'La caméra n\'est pas disponible. Tape le code affiché sur la TV.'**
  String get scanUnavailable;

  /// No description provided for @codeSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Code d\'appairage à 6 chiffres'**
  String get codeSemantics;

  /// No description provided for @connect.
  ///
  /// In fr, this message translates to:
  /// **'Connecter'**
  String get connect;

  /// No description provided for @changeTv.
  ///
  /// In fr, this message translates to:
  /// **'Changer de TV'**
  String get changeTv;

  /// No description provided for @pairing.
  ///
  /// In fr, this message translates to:
  /// **'Connexion à la TV…'**
  String get pairing;

  /// No description provided for @errorCodeInvalid.
  ///
  /// In fr, this message translates to:
  /// **'Code incorrect. Encore {count, plural, =1{1 essai} other{{count} essais}}.'**
  String errorCodeInvalid(int count);

  /// No description provided for @errorCodeInvalidNoCount.
  ///
  /// In fr, this message translates to:
  /// **'Code incorrect. Vérifie le code affiché sur la TV.'**
  String get errorCodeInvalidNoCount;

  /// No description provided for @errorCodeExpired.
  ///
  /// In fr, this message translates to:
  /// **'Ce code a expiré. Tape le nouveau code affiché sur la TV.'**
  String get errorCodeExpired;

  /// No description provided for @errorTooManyAttempts.
  ///
  /// In fr, this message translates to:
  /// **'Trop d\'essais. Réessaie dans {seconds} s.'**
  String errorTooManyAttempts(int seconds);

  /// No description provided for @errorTooManyAttemptsNoDelay.
  ///
  /// In fr, this message translates to:
  /// **'Trop d\'essais. Attends un peu, puis tape le nouveau code de la TV.'**
  String get errorTooManyAttemptsNoDelay;

  /// No description provided for @errorUnreachable.
  ///
  /// In fr, this message translates to:
  /// **'Impossible de joindre la TV à l\'adresse {address}. Vérifie que le téléphone et la TV sont sur le même Wi-Fi.'**
  String errorUnreachable(String address);

  /// No description provided for @errorUnexpected.
  ///
  /// In fr, this message translates to:
  /// **'Une erreur est survenue : {message}'**
  String errorUnexpected(String message);

  /// No description provided for @retry.
  ///
  /// In fr, this message translates to:
  /// **'Réessayer'**
  String get retry;

  /// No description provided for @checking.
  ///
  /// In fr, this message translates to:
  /// **'Connexion à ta TV…'**
  String get checking;

  /// No description provided for @guestProfile.
  ///
  /// In fr, this message translates to:
  /// **'Invité'**
  String get guestProfile;

  /// No description provided for @cancel.
  ///
  /// In fr, this message translates to:
  /// **'Annuler'**
  String get cancel;

  /// No description provided for @sessionExpired.
  ///
  /// In fr, this message translates to:
  /// **'La TV ne reconnaît plus ce téléphone. Connecte-toi de nouveau.'**
  String get sessionExpired;

  /// No description provided for @tvUnreachableTitle.
  ///
  /// In fr, this message translates to:
  /// **'Ta TV ne répond pas'**
  String get tvUnreachableTitle;

  /// No description provided for @tvUnreachableBody.
  ///
  /// In fr, this message translates to:
  /// **'Vérifie qu\'elle est allumée et sur le même Wi-Fi que ce téléphone ({address}).'**
  String tvUnreachableBody(String address);

  /// No description provided for @gloubiLabel.
  ///
  /// In fr, this message translates to:
  /// **'Gloubi, la mascotte'**
  String get gloubiLabel;

  /// No description provided for @serverLabel.
  ///
  /// In fr, this message translates to:
  /// **'Serveur : {address}'**
  String serverLabel(String address);

  /// No description provided for @changeServer.
  ///
  /// In fr, this message translates to:
  /// **'Changer'**
  String get changeServer;

  /// No description provided for @changeServerSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Changer l\'adresse du serveur, actuellement {address}'**
  String changeServerSemantics(String address);

  /// No description provided for @serverTitle.
  ///
  /// In fr, this message translates to:
  /// **'Adresse du serveur'**
  String get serverTitle;

  /// No description provided for @serverBody.
  ///
  /// In fr, this message translates to:
  /// **'L\'adresse du serveur du media center sur ton réseau.'**
  String get serverBody;

  /// No description provided for @save.
  ///
  /// In fr, this message translates to:
  /// **'Enregistrer'**
  String get save;

  /// No description provided for @resetServer.
  ///
  /// In fr, this message translates to:
  /// **'Adresse par défaut ({address})'**
  String resetServer(String address);

  /// No description provided for @orTypeCode.
  ///
  /// In fr, this message translates to:
  /// **'ou tape le code'**
  String get orTypeCode;

  /// No description provided for @qrNoCode.
  ///
  /// In fr, this message translates to:
  /// **'QR code lu. Tape maintenant le code à 6 chiffres affiché sous le QR.'**
  String get qrNoCode;

  /// No description provided for @connectingServer.
  ///
  /// In fr, this message translates to:
  /// **'Connexion au serveur…'**
  String get connectingServer;

  /// No description provided for @serverOfflineTitle.
  ///
  /// In fr, this message translates to:
  /// **'Serveur injoignable'**
  String get serverOfflineTitle;

  /// No description provided for @serverOfflineBody.
  ///
  /// In fr, this message translates to:
  /// **'Impossible de joindre le serveur à l\'adresse {address}. Vérifie l\'adresse et que le téléphone est sur le même Wi-Fi.'**
  String serverOfflineBody(String address);

  /// No description provided for @changeAddress.
  ///
  /// In fr, this message translates to:
  /// **'Changer l\'adresse'**
  String get changeAddress;

  /// No description provided for @typeCodeInstead.
  ///
  /// In fr, this message translates to:
  /// **'Taper le code'**
  String get typeCodeInstead;

  /// No description provided for @profile.
  ///
  /// In fr, this message translates to:
  /// **'Profil'**
  String get profile;

  /// No description provided for @switchProfile.
  ///
  /// In fr, this message translates to:
  /// **'Changer de profil'**
  String get switchProfile;

  /// No description provided for @pairingMenu.
  ///
  /// In fr, this message translates to:
  /// **'Appairage'**
  String get pairingMenu;

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

  /// No description provided for @noProfile.
  ///
  /// In fr, this message translates to:
  /// **'Choisir un profil'**
  String get noProfile;

  /// No description provided for @keyUp.
  ///
  /// In fr, this message translates to:
  /// **'Haut'**
  String get keyUp;

  /// No description provided for @keyDown.
  ///
  /// In fr, this message translates to:
  /// **'Bas'**
  String get keyDown;

  /// No description provided for @keyLeft.
  ///
  /// In fr, this message translates to:
  /// **'Gauche'**
  String get keyLeft;

  /// No description provided for @keyRight.
  ///
  /// In fr, this message translates to:
  /// **'Droite'**
  String get keyRight;

  /// No description provided for @keyOk.
  ///
  /// In fr, this message translates to:
  /// **'OK'**
  String get keyOk;

  /// No description provided for @keyBack.
  ///
  /// In fr, this message translates to:
  /// **'Retour'**
  String get keyBack;

  /// No description provided for @keyHome.
  ///
  /// In fr, this message translates to:
  /// **'Accueil'**
  String get keyHome;

  /// No description provided for @whoIsWatching.
  ///
  /// In fr, this message translates to:
  /// **'Qui regarde ?'**
  String get whoIsWatching;

  /// No description provided for @currentProfile.
  ///
  /// In fr, this message translates to:
  /// **'Profil actuel'**
  String get currentProfile;

  /// No description provided for @profileLocked.
  ///
  /// In fr, this message translates to:
  /// **'Protégé par un code PIN'**
  String get profileLocked;

  /// No description provided for @pinTitle.
  ///
  /// In fr, this message translates to:
  /// **'Code PIN de {name}'**
  String pinTitle(String name);

  /// No description provided for @pinSemantics.
  ///
  /// In fr, this message translates to:
  /// **'Code PIN à 4 chiffres'**
  String get pinSemantics;

  /// No description provided for @pinInvalid.
  ///
  /// In fr, this message translates to:
  /// **'PIN incorrect. Encore {count, plural, =1{1 essai} other{{count} essais}}.'**
  String pinInvalid(int count);

  /// No description provided for @pinInvalidNoCount.
  ///
  /// In fr, this message translates to:
  /// **'PIN incorrect.'**
  String get pinInvalidNoCount;

  /// No description provided for @pinLocked.
  ///
  /// In fr, this message translates to:
  /// **'Trop d\'essais. Réessaie dans {seconds} s.'**
  String pinLocked(int seconds);

  /// No description provided for @pinLockedNoDelay.
  ///
  /// In fr, this message translates to:
  /// **'Trop d\'essais. Réessaie plus tard.'**
  String get pinLockedNoDelay;

  /// No description provided for @profilesError.
  ///
  /// In fr, this message translates to:
  /// **'Impossible de charger les comptes.'**
  String get profilesError;

  /// No description provided for @pairingConfirmTitle.
  ///
  /// In fr, this message translates to:
  /// **'Revenir à l\'appairage ?'**
  String get pairingConfirmTitle;

  /// No description provided for @pairingConfirmBody.
  ///
  /// In fr, this message translates to:
  /// **'La TV revient à son écran d\'appairage et ce téléphone s\'en déconnecte. Il faudra de nouveau scanner le QR code ou taper le code.'**
  String get pairingConfirmBody;

  /// No description provided for @confirm.
  ///
  /// In fr, this message translates to:
  /// **'Continuer'**
  String get confirm;

  /// No description provided for @infoMenu.
  ///
  /// In fr, this message translates to:
  /// **'Infos'**
  String get infoMenu;

  /// No description provided for @infoTitle.
  ///
  /// In fr, this message translates to:
  /// **'Infos'**
  String get infoTitle;

  /// No description provided for @infoServer.
  ///
  /// In fr, this message translates to:
  /// **'Serveur'**
  String get infoServer;

  /// No description provided for @infoPhone.
  ///
  /// In fr, this message translates to:
  /// **'Ce téléphone'**
  String get infoPhone;

  /// No description provided for @infoProfile.
  ///
  /// In fr, this message translates to:
  /// **'Profil actif'**
  String get infoProfile;

  /// No description provided for @infoVersion.
  ///
  /// In fr, this message translates to:
  /// **'Version de l\'app'**
  String get infoVersion;

  /// No description provided for @changeServerConfirmTitle.
  ///
  /// In fr, this message translates to:
  /// **'Changer de serveur ?'**
  String get changeServerConfirmTitle;

  /// No description provided for @changeServerConfirmBody.
  ///
  /// In fr, this message translates to:
  /// **'Ce téléphone sera déconnecté de la TV actuelle, puis se connectera au nouveau serveur.'**
  String get changeServerConfirmBody;

  /// No description provided for @keyboardTitle.
  ///
  /// In fr, this message translates to:
  /// **'Écrire sur la TV'**
  String get keyboardTitle;

  /// No description provided for @keyboardDone.
  ///
  /// In fr, this message translates to:
  /// **'Valider'**
  String get keyboardDone;

  /// No description provided for @keyboardClose.
  ///
  /// In fr, this message translates to:
  /// **'Fermer le clavier'**
  String get keyboardClose;

  /// No description provided for @languageToggle.
  ///
  /// In fr, this message translates to:
  /// **'Langue : passer la TV et le téléphone en {language}'**
  String languageToggle(String language);
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
