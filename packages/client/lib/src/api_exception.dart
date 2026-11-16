/// Erreur renvoyée par le serveur ({"error": {...}}) ou erreur réseau (status 0).
class ApiException implements Exception {
  const ApiException({
    required this.status,
    required this.code,
    required this.message,
    this.details,
  });

  /// Code HTTP, ou 0 si le serveur est injoignable.
  final int status;

  /// Code machine : "pairing_code_invalid", "network_error", "timeout"...
  final String code;
  final String message;
  final Map<String, dynamic>? details;

  bool get isNetworkError => status == 0;

  @override
  String toString() => 'ApiException($status, $code): $message';
}
