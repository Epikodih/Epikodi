class Qr {
  const Qr({required this.payload, required this.format});

  factory Qr.fromJson(Map<String, dynamic> json) =>
      Qr(payload: json['payload'] as String, format: json['format'] as String);

  /// Texte à encoder dans le QR code (c'est à l'app TV de le dessiner).
  final String payload;
  final String format;
}

class Pairing {
  const Pairing({
    required this.code,
    required this.expiresAt,
    required this.expiresInS,
    required this.attemptsLeft,
    required this.qr,
  });

  factory Pairing.fromJson(Map<String, dynamic> json) => Pairing(
        code: json['code'] as String,
        expiresAt: DateTime.parse(json['expires_at'] as String),
        expiresInS: json['expires_in_s'] as int,
        attemptsLeft: json['attempts_left'] as int,
        qr: Qr.fromJson(json['qr'] as Map<String, dynamic>),
      );

  final String code;
  final DateTime expiresAt;
  final int expiresInS;
  final int attemptsLeft;
  final Qr qr;
}
