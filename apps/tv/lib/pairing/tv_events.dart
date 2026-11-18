import 'dart:async';
import 'dart:convert';
import 'dart:io';

/// Écoute le flux temps réel du serveur (GET /api/tv/events, Server-Sent Events).
///
/// Le serveur envoie l'état dès l'ouverture, puis à chaque changement :
///   {"phase": "pairing" | "paired" | "ready", "profiles": [...], ...}
/// Se reconnecte tout seul si la connexion tombe. Aucune dépendance (dart:io).
class TvEvents {
  TvEvents({
    required this.baseUrl,
    required this.sessionId,
    required this.onEvent,
    this.onSessionLost,
    this.retryDelay = const Duration(seconds: 2),
  });

  final String baseUrl;
  final String sessionId;

  /// Pause avant de se reconnecter quand la connexion tombe.
  final Duration retryDelay;

  /// Appelé à chaque événement, avec le JSON décodé.
  final void Function(Map<String, dynamic> data) onEvent;

  /// Appelé si le serveur ne connaît plus la session (404) : il faut refaire connect.
  final void Function()? onSessionLost;

  final _http = HttpClient();
  bool _closed = false;

  void start() => _run();

  void close() {
    _closed = true;
    _http.close(force: true);
  }

  Future<void> _run() async {
    while (!_closed) {
      try {
        final uri = Uri.parse('$baseUrl/api/tv/events')
            .replace(queryParameters: {'tv_session_id': sessionId});
        final req = await _http.getUrl(uri);
        req.headers.set(HttpHeaders.acceptHeader, 'text/event-stream');
        final res = await req.close();

        if (res.statusCode == 404) {
          onSessionLost?.call();
          return;
        }

        await for (final line
            in res.transform(utf8.decoder).transform(const LineSplitter())) {
          if (_closed) return;
          if (line.startsWith('data:')) {
            final json = _decode(line.substring(5).trim());
            if (json is Map<String, dynamic>) onEvent(json);
          }
        }
      } catch (_) {
        // serveur arrêté ou réseau coupé : on réessaie ci-dessous
      }
      if (!_closed) await Future<void>.delayed(retryDelay);
    }
  }

  /// Une ligne `data:` illisible est ignorée (sans couper le flux).
  static Object? _decode(String text) {
    try {
      return jsonDecode(text);
    } on FormatException {
      return null;
    }
  }
}
