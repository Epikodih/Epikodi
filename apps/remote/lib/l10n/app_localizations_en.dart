// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'GloubiTV Remote';

  @override
  String get welcomeTitle => 'Connect to your TV';

  @override
  String get welcomeBody =>
      'Scan the QR code shown on the TV, or type the 6-digit code.';

  @override
  String get scanQr => 'Scan the QR code on the TV';

  @override
  String get addressHint => '192.168.1.42';

  @override
  String get addressInvalid =>
      'Invalid address. Example: 192.168.1.42 or 192.168.1.42:7878';

  @override
  String get scanTitle => 'Point at the QR code on the TV';

  @override
  String get scanNotTv => 'This QR code does not come from a GloubiTV TV.';

  @override
  String get scanNoPermission =>
      'GloubiTV needs the camera to read the QR code. Allow it in settings, or type the code shown on the TV.';

  @override
  String get scanUnavailable =>
      'The camera is not available. Type the code shown on the TV.';

  @override
  String get codeSemantics => '6-digit pairing code';

  @override
  String get connect => 'Connect';

  @override
  String get changeTv => 'Change TV';

  @override
  String get pairing => 'Connecting to the TV…';

  @override
  String errorCodeInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count tries',
      one: '1 try',
    );
    return 'Wrong code. $_temp0 left.';
  }

  @override
  String get errorCodeInvalidNoCount =>
      'Wrong code. Check the code shown on the TV.';

  @override
  String get errorCodeExpired =>
      'This code has expired. Type the new code shown on the TV.';

  @override
  String errorTooManyAttempts(int seconds) {
    return 'Too many tries. Try again in $seconds s.';
  }

  @override
  String get errorTooManyAttemptsNoDelay =>
      'Too many tries. Wait a bit, then type the new code from the TV.';

  @override
  String errorUnreachable(String address) {
    return 'Cannot reach the TV at $address. Make sure the phone and the TV are on the same Wi-Fi.';
  }

  @override
  String errorUnexpected(String message) {
    return 'Something went wrong: $message';
  }

  @override
  String get retry => 'Try again';

  @override
  String get checking => 'Connecting to your TV…';

  @override
  String get guestProfile => 'Guest';

  @override
  String get cancel => 'Cancel';

  @override
  String get sessionExpired =>
      'The TV no longer knows this phone. Connect again.';

  @override
  String get tvUnreachableTitle => 'Your TV does not answer';

  @override
  String tvUnreachableBody(String address) {
    return 'Make sure it is on and on the same Wi-Fi as this phone ($address).';
  }

  @override
  String get gloubiLabel => 'Gloubi, the mascot';

  @override
  String serverLabel(String address) {
    return 'Server: $address';
  }

  @override
  String get changeServer => 'Change';

  @override
  String changeServerSemantics(String address) {
    return 'Change the server address, currently $address';
  }

  @override
  String get serverTitle => 'Server address';

  @override
  String get serverBody =>
      'The address of the media center server on your network.';

  @override
  String get save => 'Save';

  @override
  String resetServer(String address) {
    return 'Default address ($address)';
  }

  @override
  String get orTypeCode => 'or type the code';

  @override
  String get qrNoCode =>
      'QR code read. Now type the 6-digit code shown under the QR.';

  @override
  String get connectingServer => 'Connecting to the server…';

  @override
  String get serverOfflineTitle => 'Server unreachable';

  @override
  String serverOfflineBody(String address) {
    return 'Cannot reach the server at $address. Check the address and that the phone is on the same Wi-Fi.';
  }

  @override
  String get changeAddress => 'Change the address';

  @override
  String get typeCodeInstead => 'Type the code';

  @override
  String get profile => 'Profile';

  @override
  String get switchProfile => 'Switch profile';

  @override
  String get pairingMenu => 'Pairing';

  @override
  String profileMenuSemantics(String name) {
    return 'Profile menu of $name';
  }

  @override
  String get closeMenu => 'Close the menu';

  @override
  String get noProfile => 'Choose a profile';

  @override
  String get keyUp => 'Up';

  @override
  String get keyDown => 'Down';

  @override
  String get keyLeft => 'Left';

  @override
  String get keyRight => 'Right';

  @override
  String get keyOk => 'OK';

  @override
  String get keyBack => 'Back';

  @override
  String get keyHome => 'Home';

  @override
  String get whoIsWatching => 'Who\'s watching?';

  @override
  String get currentProfile => 'Current profile';

  @override
  String get profileLocked => 'Protected by a PIN';

  @override
  String pinTitle(String name) {
    return 'PIN for $name';
  }

  @override
  String get pinSemantics => '4-digit PIN';

  @override
  String pinInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count tries',
      one: '1 try',
    );
    return 'Wrong PIN. $_temp0 left.';
  }

  @override
  String get pinInvalidNoCount => 'Wrong PIN.';

  @override
  String pinLocked(int seconds) {
    return 'Too many tries. Try again in $seconds s.';
  }

  @override
  String get pinLockedNoDelay => 'Too many tries. Try again later.';

  @override
  String get profilesError => 'Could not load the accounts.';

  @override
  String get pairingConfirmTitle => 'Back to pairing?';

  @override
  String get pairingConfirmBody =>
      'The TV goes back to its pairing screen and this phone disconnects from it. You will need to scan the QR code or type the code again.';

  @override
  String get confirm => 'Continue';

  @override
  String get infoMenu => 'Info';

  @override
  String get infoTitle => 'Info';

  @override
  String get infoServer => 'Server';

  @override
  String get infoPhone => 'This phone';

  @override
  String get infoProfile => 'Active profile';

  @override
  String get infoVersion => 'App version';

  @override
  String get changeServerConfirmTitle => 'Change server?';

  @override
  String get changeServerConfirmBody =>
      'This phone will be disconnected from the current TV, then connect to the new server.';

  @override
  String get keyboardTitle => 'Type on the TV';

  @override
  String get keyboardDone => 'Done';

  @override
  String get keyboardClose => 'Close the keyboard';

  @override
  String languageToggle(String language) {
    return 'Language: switch the TV and the phone to $language';
  }
}
