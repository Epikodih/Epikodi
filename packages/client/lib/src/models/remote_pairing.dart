/// Port par défaut du serveur du media center.
const defaultServerPort = 7878;

/// TV à laquelle la télécommande veut se connecter : son adresse sur le
/// réseau local, lue dans le QR affiché par la TV ou saisie à la main.
///
/// Le QR (voir `fixtures/tv/connect_response.json`) contient
/// `mediacenter://pair?host=192.168.1.42&port=7878&sid=a3f9c2e1&code=482916`.
/// Le code du QR remplace le code tapé : scanner suffit pour appairer. Sans
/// code : l'utilisateur tape ensuite le code affiché sous le QR.
class PairTarget {
  const PairTarget({
    required this.host,
    this.port = defaultServerPort,
    this.sessionId,
    this.code,
  });

  final String host;
  final int port;

  /// Session TV (`sid` du QR). Absente quand l'adresse est saisie à la main.
  final String? sessionId;

  /// Code d'appairage du QR (`code=`) : scanner remplace la saisie.
  /// `null` si le QR n'en contient pas (l'utilisateur tape alors le code).
  final String? code;

  /// Adresse HTTP du serveur, ex. `http://192.168.1.42:7878`.
  String get baseUrl => 'http://${host.contains(':') ? '[$host]' : host}:$port';

  /// Texte affiché à l'utilisateur, ex. `192.168.1.42:7878`.
  String get label =>
      port == defaultServerPort ? host : '$host:$port';

  /// Lit le contenu d'un QR d'appairage. `null` si ce n'est pas un QR de TV.
  static PairTarget? fromQr(String payload) {
    final uri = Uri.tryParse(payload.trim());
    if (uri == null || uri.scheme != 'mediacenter' || uri.host != 'pair') {
      return null;
    }
    final host = uri.queryParameters['host'];
    if (host == null || !_validHost(host)) return null;
    final portText = uri.queryParameters['port'];
    final port = portText == null ? defaultServerPort : int.tryParse(portText);
    if (port == null || port <= 0 || port > 65535) return null;
    final sid = uri.queryParameters['sid'];
    final code = uri.queryParameters['code'];
    return PairTarget(
      host: host,
      port: port,
      sessionId: (sid == null || sid.isEmpty) ? null : sid,
      code: (code != null && RegExp(r'^\d{6}$').hasMatch(code)) ? code : null,
    );
  }

  /// Lit une adresse tapée à la main : `192.168.1.42`, `192.168.1.42:7878`,
  /// `http://tv.local:7878`, ou directement le texte d'un QR.
  /// `null` si l'adresse n'est pas valide.
  static PairTarget? parse(String input) {
    final text = input.trim();
    if (text.isEmpty) return null;
    if (text.startsWith('mediacenter://')) return fromQr(text);
    final withScheme = text.contains('://') ? text : 'http://$text';
    final uri = Uri.tryParse(withScheme);
    if (uri == null ||
        (uri.scheme != 'http' && uri.scheme != 'https') ||
        !_validHost(uri.host) ||
        (uri.path.isNotEmpty && uri.path != '/')) {
      return null;
    }
    final port = uri.hasPort ? uri.port : defaultServerPort;
    if (port <= 0 || port > 65535) return null;
    return PairTarget(host: uri.host, port: port);
  }

  /// Nom ou IPv4 (lettres, chiffres, points, tirets), ou IPv6.
  static bool _validHost(String host) =>
      RegExp(r'^[A-Za-z0-9]([A-Za-z0-9.-]*[A-Za-z0-9])?$').hasMatch(host) ||
      (host.contains(':') && RegExp(r'^[0-9A-Fa-f:.]+$').hasMatch(host));

  @override
  bool operator ==(Object other) =>
      other is PairTarget &&
      other.host == host &&
      other.port == port &&
      other.sessionId == sessionId &&
      other.code == code;

  @override
  int get hashCode => Object.hash(host, port, sessionId, code);

  @override
  String toString() => 'PairTarget($host:$port, sid: $sessionId)';
}

/// Le téléphone, tel qu'il se présente au serveur (`pairing/request.json`).
class RemoteDevice {
  const RemoteDevice({
    required this.name,
    required this.installId,
    required this.platform,
  });

  /// Nom lisible, ex. « Pixel 7 ».
  final String name;

  /// Identifiant tiré au hasard à la première ouverture de l'app, gardé
  /// ensuite : le serveur reconnaît le même téléphone s'il se réappaire.
  final String installId;

  /// `android` (ou `ios` plus tard).
  final String platform;

  Map<String, dynamic> toJson() => {
        'name': name,
        'install_id': installId,
        'platform': platform,
      };
}

/// Réponse de `POST /api/remote/pair` (`pairing/response.json`).
class PairResponse {
  const PairResponse({
    required this.token,
    required this.tokenType,
    required this.expiresAt,
    required this.deviceId,
    required this.deviceName,
  });

  factory PairResponse.fromJson(Map<String, dynamic> json) {
    final device = json['device'] as Map<String, dynamic>;
    final expires = json['expires_at'] as String?;
    return PairResponse(
      token: json['token'] as String,
      tokenType: json['token_type'] as String,
      expiresAt: expires == null ? null : DateTime.parse(expires),
      deviceId: device['id'] as int,
      deviceName: device['name'] as String,
    );
  }

  /// Token à envoyer ensuite dans `Authorization: Bearer <token>`.
  final String token;
  final String tokenType;

  /// `null` si le token n'expire pas.
  final DateTime? expiresAt;
  final int deviceId;
  final String deviceName;
}

/// Profil actif sur la TV (`profile` de `session/current.json`).
class SessionProfile {
  const SessionProfile({
    required this.kind,
    required this.id,
    required this.name,
    required this.icon,
    required this.language,
    required this.age,
    required this.isRestricted,
  });

  factory SessionProfile.fromJson(Map<String, dynamic> json) => SessionProfile(
        kind: json['kind'] as String? ?? 'account',
        id: json['id'] as int?,
        name: json['name'] as String,
        icon: json['icon'] as String?,
        language: json['language'] as String? ?? 'fr',
        age: json['age'] as int? ?? 0,
        isRestricted: json['is_restricted'] as bool? ?? true,
      );

  /// `account` ou `guest`.
  final String kind;

  /// `null` pour l'invité.
  final int? id;
  final String name;
  final String? icon;
  final String language;
  final int age;
  final bool isRestricted;

  bool get isGuest => kind == 'guest';
}

/// Réponse de `GET /api/session` (`session/current.json`).
class RemoteSession {
  const RemoteSession({
    required this.deviceId,
    required this.deviceName,
    required this.pairedAt,
    required this.profile,
  });

  factory RemoteSession.fromJson(Map<String, dynamic> json) {
    final device = json['device'] as Map<String, dynamic>;
    final profile = json['profile'];
    final pairedAt = device['paired_at'] as String?;
    return RemoteSession(
      deviceId: device['id'] as int,
      deviceName: device['name'] as String,
      pairedAt: pairedAt == null ? null : DateTime.parse(pairedAt),
      profile: profile is Map<String, dynamic>
          ? SessionProfile.fromJson(profile)
          : null,
    );
  }

  final int deviceId;
  final String deviceName;
  final DateTime? pairedAt;

  /// `null` tant qu'aucun profil n'est choisi sur la TV.
  final SessionProfile? profile;
}

/// Un compte de `GET /api/profiles` (`profiles/list.json`).
class ProfileSummary {
  const ProfileSummary({
    required this.id,
    required this.name,
    required this.icon,
    required this.language,
    required this.age,
    required this.isRestricted,
    required this.requiresPin,
  });

  factory ProfileSummary.fromJson(Map<String, dynamic> json) => ProfileSummary(
        id: json['id'] as int,
        name: json['name'] as String,
        icon: json['icon'] as String?,
        language: json['language'] as String? ?? 'fr',
        age: json['age'] as int? ?? 0,
        isRestricted: json['is_restricted'] as bool? ?? true,
        requiresPin: json['requires_pin'] as bool? ?? false,
      );

  final int id;
  final String name;
  final String? icon;
  final String language;
  final int age;
  final bool isRestricted;

  /// Vrai : il faut le PIN du compte pour le choisir.
  final bool requiresPin;
}

/// Réponse de `GET /api/profiles` : tout ce qu'il faut pour choisir un compte.
class ProfileList {
  const ProfileList({
    required this.guestAvailable,
    required this.canCreate,
    required this.profiles,
  });

  factory ProfileList.fromJson(Map<String, dynamic> json) {
    final guest = json['guest'];
    return ProfileList(
      guestAvailable:
          guest is Map<String, dynamic> && (guest['available'] as bool? ?? false),
      canCreate: json['can_create'] as bool? ?? false,
      profiles: [
        for (final p in (json['profiles'] as List? ?? const []))
          ProfileSummary.fromJson(p as Map<String, dynamic>),
      ],
    );
  }

  final bool guestAvailable;
  final bool canCreate;
  final List<ProfileSummary> profiles;
}
