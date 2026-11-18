// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get connectingToServer => 'Connecting to server';

  @override
  String get retry => 'Retry';

  @override
  String get retryConnectionLabel => 'Try connecting to the server again';

  @override
  String unexpectedError(String details) {
    return 'Unexpected error: $details';
  }

  @override
  String get createAccount => 'Create an account';

  @override
  String get createAccountButton => 'Create account';

  @override
  String get creating => 'Creating...';

  @override
  String get back => 'Back';

  @override
  String get name => 'Name';

  @override
  String get nameHint => 'First name or nickname';

  @override
  String get enterName => 'Enter a name.';

  @override
  String get choosePhoto => 'Choose my photo';

  @override
  String get imagePickError => 'Unable to pick an image here.';

  @override
  String avatarIcon(int number) {
    return 'Icon $number';
  }

  @override
  String avatarIconSelected(int number) {
    return 'Icon $number, selected';
  }

  @override
  String get age => 'Age';

  @override
  String ageYears(int age) {
    return '$age years old';
  }

  @override
  String get decreaseAge => 'Decrease age';

  @override
  String get increaseAge => 'Increase age';

  @override
  String get language => 'Language';

  @override
  String get childProfile => 'Child profile';

  @override
  String get childProfileSemantic => 'Child profile (parental control)';

  @override
  String get yes => 'Yes';

  @override
  String get no => 'No';

  @override
  String get noContentBlocked => 'No content blocked';

  @override
  String get noContentBlockedAdult => 'No content blocked (18 and over)';

  @override
  String blocksRatingAndAbove(int rating) {
    return 'Blocks $rating+ and above';
  }

  @override
  String get pairingTitle => 'Connect the remote';

  @override
  String get pairingInstruction =>
      'Scan the QR code or enter the code in the app.';

  @override
  String get newCode => 'New code';

  @override
  String newCodeCooldown(int seconds) {
    return 'New code ($seconds s)';
  }

  @override
  String get newCodeSemantics => 'Generate a new code and a new QR code';

  @override
  String get newCodeCoolingSemantics => 'New code, available in a few seconds';

  @override
  String expiresIn(String time) {
    return 'Expires in $time';
  }

  @override
  String expiresSemantics(int minutes) {
    return 'The code expires in $minutes minutes';
  }

  @override
  String pairingCodeSemantics(String code) {
    return 'Pairing code: $code';
  }

  @override
  String get pairingQrSemantics => 'Pairing QR code';

  @override
  String get switchLanguageLabel => 'Switch language';

  @override
  String get noAccountYet => 'No account yet';

  @override
  String get whoIsWatching => 'Who\'s watching?';

  @override
  String get createOrGuest => 'Create an account or watch as a guest';

  @override
  String get createCard => 'Create';

  @override
  String get guestCard => 'Guest';

  @override
  String get continueAsGuest => 'Continue as guest';

  @override
  String accountSemantics(String name) {
    return 'Account $name';
  }

  @override
  String accountLockedSemantics(String name) {
    return 'Account $name, protected by a PIN code';
  }

  @override
  String get pairingButton => 'Pairing';

  @override
  String get backToPairingLabel => 'Back to the pairing screen';

  @override
  String get profile => 'Profile';

  @override
  String profileMenuSemantics(String name) {
    return '$name profile menu';
  }

  @override
  String get closeMenu => 'Close menu';

  @override
  String get switchProfile => 'Switch profile';

  @override
  String get pinTitle => 'PIN required';

  @override
  String get pinSubtitle => 'Enter the PIN to change the age restriction.';

  @override
  String get pinWrong => 'Incorrect PIN.';

  @override
  String pinDigitSemantics(String digit) {
    return 'Digit $digit';
  }

  @override
  String get deleteDigit => 'Delete';

  @override
  String get cancel => 'Cancel';

  @override
  String get save => 'Save';

  @override
  String get saving => 'Saving...';

  @override
  String get pinWillBeAsked => 'The PIN will be requested when saving.';

  @override
  String get sectionMovies => 'Movies';

  @override
  String get sectionSeries => 'TV shows';

  @override
  String get sectionMusic => 'Music';

  @override
  String get sectionPodcasts => 'Podcasts';

  @override
  String get libraryLoadError => 'Unable to load the library.';

  @override
  String get noMediaForProfile => 'No media available for this profile.';

  @override
  String get guestName => 'Guest';

  @override
  String get playerPlay => 'Play';

  @override
  String get playerPause => 'Pause';

  @override
  String get playerRewind => 'Rewind 10 seconds';

  @override
  String get playerForward => 'Forward 30 seconds';

  @override
  String get playerLoading => 'Loading…';

  @override
  String get playerPaused => 'Paused';

  @override
  String get playerEnded => 'Playback finished';

  @override
  String get playerPositionLabel => 'Playback position';

  @override
  String playerPosition(String position, String duration) {
    return '$position of $duration';
  }

  @override
  String playerError(String details) {
    return 'Unable to play: $details';
  }

  @override
  String get playerTracks => 'Audio and subtitles';

  @override
  String get playerAudio => 'Audio';

  @override
  String get playerSubtitles => 'Subtitles';

  @override
  String get playerSubtitlesOff => 'Off';

  @override
  String get playerAudioDescription => 'Audio description';

  @override
  String playerTrackNumber(int number) {
    return 'Track $number';
  }

  @override
  String get playerNoTrack => 'No track in this file';

  @override
  String get subtitleSize => 'Subtitle size';

  @override
  String get subtitleSizeSmall => 'Small';

  @override
  String get subtitleSizeNormal => 'Normal';

  @override
  String get subtitleSizeLarge => 'Large';

  @override
  String get subtitleSizeExtraLarge => 'Extra large';

  @override
  String get subtitleColor => 'Text colour';

  @override
  String get subtitleColorWhite => 'White';

  @override
  String get subtitleColorYellow => 'Yellow';

  @override
  String get subtitleColorCyan => 'Cyan';

  @override
  String get subtitleBackground => 'Background';

  @override
  String get subtitleBackgroundNone => 'None';

  @override
  String get subtitleBackgroundTranslucent => 'Semi-transparent';

  @override
  String get subtitleBackgroundOpaque => 'Black';

  @override
  String get subtitlePreview => 'Subtitle preview';

  @override
  String get mediaUnavailable => 'Unavailable';
}
