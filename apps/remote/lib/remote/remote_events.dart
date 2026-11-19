import 'dart:async';
import 'dart:convert';
import 'dart:io';

/// Champ texte qui a le focus sur la TV : le téléphone ouvre son clavier.
/// Événement `{"type": "text_input", "active": true, ...}` du serveur.
class TextInputRequest {
  const TextInputRequest({
    required this.id,
    required this.label,
    required this.value,
    this.obscure = false,
    this.number = false,
    this.maxLength,
  });

  /// `null` si l'événement n'annonce pas de champ (`active: false`).
  static TextInputRequest? fromEvent(Map<String, dynamic> json) {
    if (json['type'] != 'text_input' || json['active'] != true) return null;
    return TextInputRequest(
      id: (json['id'] as num?)?.toInt() ?? 0,
      label: json['label'] as String? ?? '',
      value: json['value'] as String? ?? '',
      obscure: json['obscure'] == true,
      number: json['keyboard'] == 'number',
      maxLength: (json['max_length'] as num?)?.toInt(),
    );
  }

  /// Identifie le champ de la TV (change quand le focus passe à un autre).
  final int id;
  final String label;
  final String value;
  final bool obscure;
  final bool number;
  final int? maxLength;
}

/// Flux d'événements du serveur vers le téléphone.
abstract class RemoteEventsSource {
  void start();
  void close();
}

/// `GET /api/remote/events` (Server-Sent Events, route provisoire du faux
/// serveur `test_api.py`). Se reconnecte tout seul si la connexion tombe.
class SseRemoteEvents implements RemoteEventsSource {
  SseRemoteEvents({
    required this.baseUrl,
    required this.token,
    required this.onEvent,
    this.retryDelay = const Duration(seconds: 3),
  });

  final String baseUrl;
  final String? token;
  final void Function(Map<String, dynamic> event) onEvent;
  final Duration retryDelay;

  final _http = HttpClient();
  bool _closed = false;

  @override
  void start() => _run();

  @override
  void close() {
    _closed = true;
    _http.close(force: true);
  }

  Future<void> _run() async {
    final base = baseUrl.replaceFirst(RegExp(r'/+$'), '');
    while (!_closed) {
      try {
        final req = await _http.getUrl(Uri.parse('$base/api/remote/events'));
        req.headers.set(HttpHeaders.acceptHeader, 'text/event-stream');
        if (token != null) {
          req.headers.set(HttpHeaders.authorizationHeader, 'Bearer $token');
        }
        final res = await req.close();
        if (res.statusCode == 200) {
          await for (final line in res
              .transform(utf8.decoder)
              .transform(const LineSplitter())) {
            if (_closed) return;
            if (!line.startsWith('data:')) continue;
            try {
              final json = jsonDecode(line.substring(5).trim());
              if (json is Map<String, dynamic>) onEvent(json);
            } on FormatException {
              // ligne illisible : ignorée
            }
          }
        } else {
          await res.drain<void>();
        }
      } catch (_) {
        // serveur arrêté, réseau coupé, route absente : on réessaie
      }
      if (!_closed) await Future<void>.delayed(retryDelay);
    }
  }
}
