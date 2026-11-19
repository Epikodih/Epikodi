import 'package:epikodi_client/epikodi_client.dart';
import 'package:flutter/foundation.dart';

/// Touches de la télécommande.
enum TvKey { up, down, left, right, ok, back, home }

/// Raccourcis du menu : ouvrent directement un écran de la TV, sans passer
/// par les flèches.
enum TvShortcut { profile }

/// Envoi des commandes à la TV.
///
/// Les fixtures ne décrivent pas encore de route pour les touches et les
/// raccourcis : c'est au vrai backend de la définir. En attendant,
/// [HttpTvCommands] utilise des routes provisoires, servies par le faux
/// serveur (`test_api.py`). Si le backend choisit d'autres routes, il
/// suffira de changer ce fichier, sans toucher aux écrans.
abstract class TvCommands {
  Future<void> key(TvKey key);
  Future<void> shortcut(TvShortcut shortcut);

  /// Texte tapé pour le champ qui a le focus sur la TV. [done] : validé
  /// (comme Entrée).
  Future<void> text(String value, {bool done = false});

  /// Langue de la TV : `fr` ou `en`.
  Future<void> language(String code);

  /// Désappaire ce téléphone : la TV revient à son écran d'appairage.
  Future<void> unpair();
}

/// En attendant la route du serveur : rien n'est envoyé.
class PendingTvCommands implements TvCommands {
  const PendingTvCommands();

  @override
  Future<void> key(TvKey key) async {
    debugPrint('Télécommande : touche ${key.name} (pas encore de route API)');
  }

  @override
  Future<void> shortcut(TvShortcut shortcut) async {
    debugPrint(
        'Télécommande : raccourci ${shortcut.name} (pas encore de route API)');
  }

  @override
  Future<void> text(String value, {bool done = false}) async {}

  @override
  Future<void> language(String code) async {}

  @override
  Future<void> unpair() async {}
}

/// Routes provisoires (faux serveur `test_api.py`) :
///  - `POST /api/remote/key`      `{"key": "up"}`
///  - `POST /api/remote/shortcut` `{"shortcut": "profile"}`
///  - `POST /api/remote/text`     `{"value": "Léa", "done": false}`
///  - `POST /api/remote/unpair`   la TV revient à l'appairage
///
/// Une touche perdue n'est pas grave : les erreurs (réseau, route absente
/// sur le vrai backend) sont ignorées, la télécommande reste utilisable.
class HttpTvCommands implements TvCommands {
  const HttpTvCommands(this._client);

  final EpikodiClient _client;

  @override
  Future<void> key(TvKey key) => _send('/api/remote/key', {'key': key.name});

  @override
  Future<void> shortcut(TvShortcut shortcut) =>
      _send('/api/remote/shortcut', {'shortcut': shortcut.name});

  @override
  Future<void> text(String value, {bool done = false}) =>
      _send('/api/remote/text', {'value': value, 'done': done});

  @override
  Future<void> language(String code) => _send(
      '/api/remote/shortcut', {'shortcut': 'language', 'language': code});

  @override
  Future<void> unpair() => _send('/api/remote/unpair', const {});

  Future<void> _send(String path, Map<String, Object?> body) async {
    try {
      await _client.http.post(path, body: body);
    } on ApiException catch (e) {
      debugPrint('Télécommande : $path refusé ($e)');
    }
  }
}
