/// Outils de test : un faux serveur qui répond avec les fixtures de /fixtures.
///
///     final backend = FixtureBackend();
///     final client = EpikodiClient(baseUrl: 'http://tv.test', httpClient: backend.client);
///     await client.tv.connect(...);                // -> fixtures/tv/connect_response.json
///     backend.last('POST', '/api/tv/connect')!.body // corps envoyé, décodé
///
/// Les routes viennent de `fixtures/routes.json` : si une route ou un fichier
/// change là-bas, les tests des apps le voient tout de suite.
/// Réservé aux tests (utilise dart:io pour lire les fichiers).
library;

import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

/// Dossier `fixtures/` du dépôt, trouvé en remontant depuis le dossier courant.
Directory findFixturesDir([Directory? from]) {
  var dir = (from ?? Directory.current).absolute;
  while (true) {
    final candidate = Directory('${dir.path}${Platform.pathSeparator}fixtures');
    if (File('${candidate.path}${Platform.pathSeparator}routes.json')
        .existsSync()) {
      return candidate;
    }
    final parent = dir.parent;
    if (parent.path == dir.path) {
      throw StateError(
          'Dossier fixtures/ introuvable au-dessus de ${Directory.current.path}');
    }
    dir = parent;
  }
}

/// Lit une fixture (chemin relatif à `fixtures/`, ex. `tv/connect_response.json`).
dynamic readFixture(String relativePath, {Directory? fixturesDir}) {
  final root = fixturesDir ?? findFixturesDir();
  final file = File('${root.path}/$relativePath');
  return jsonDecode(file.readAsStringSync());
}

/// Une route décrite dans `fixtures/routes.json`.
class FixtureRoute {
  FixtureRoute(this.json)
      : method = json['method'] as String,
        path = json['path'] as String,
        status = json['status'] as int,
        _pattern = RegExp(
            '^${(json['path'] as String).replaceAll(RegExp(r'\{[^/]+\}'), '[^/]+')}\$');

  final Map<String, dynamic> json;
  final String method;
  final String path;
  final int status;
  final RegExp _pattern;

  String? get request => json['request'] as String?;
  String? get response => json['response'] as String?;

  /// Codes d'erreur que la route peut renvoyer (`errors[].code`).
  List<String> get errorCodes => [
        for (final e in (json['errors'] as List? ?? const []))
          if ((e as Map)['code'] != null) e['code'] as String,
      ];

  bool matches(String method, String path) =>
      method == this.method && _pattern.hasMatch(path);
}

/// Requête reçue par le faux serveur.
class RecordedRequest {
  RecordedRequest(this.raw, this.body);

  final http.Request raw;

  /// Corps JSON décodé (null si vide).
  final Object? body;

  String get method => raw.method;
  String get path => raw.url.path;
  Map<String, String> get query => raw.url.queryParameters;
  Map<String, String> get headers => raw.headers;

  Map<String, dynamic> get json => body as Map<String, dynamic>;
}

/// Réponse personnalisée : statut + corps JSON.
typedef FixtureReply = (int status, Object? body);

class FixtureBackend {
  FixtureBackend({Directory? fixturesDir})
      : fixturesDir = fixturesDir ?? findFixturesDir() {
    final spec = readFixture('routes.json', fixturesDir: this.fixturesDir)
        as Map<String, dynamic>;
    routes = [
      for (final r in spec['routes'] as List)
        FixtureRoute(r as Map<String, dynamic>),
    ];
  }

  final Directory fixturesDir;
  late final List<FixtureRoute> routes;

  /// Toutes les requêtes reçues, dans l'ordre.
  final requests = <RecordedRequest>[];

  /// Réponses personnalisées, par (méthode, chemin avec `{id}`).
  final _overrides =
      <(String, String), FixtureReply Function(RecordedRequest)>{};

  static RegExp _compile(String path) =>
      RegExp('^${path.replaceAll(RegExp(r'\{[^/]+\}'), '[^/]+')}\$');

  /// Client HTTP à donner à `EpikodiClient(httpClient: ...)`.
  late final http.Client client = MockClient(_handle);

  /// Remplace la réponse d'une route (prioritaire sur la fixture).
  /// [path] accepte `{id}` comme dans routes.json.
  void on(String method, String path,
      FixtureReply Function(RecordedRequest r) reply) {
    _overrides[(method, path)] = reply;
  }

  /// Fait répondre une route avec une erreur au format `errors/error.json`.
  void fail(
    String method,
    String path, {
    required int status,
    required String code,
    String message = 'Erreur de test.',
    Map<String, dynamic>? details,
  }) {
    on(method, path,
        (_) => (status, errorBody(code, message: message, details: details)));
  }

  /// Retire une réponse personnalisée : la route renvoie de nouveau sa fixture.
  void reset(String method, String path) {
    _overrides.remove((method, path));
  }

  static Map<String, dynamic> errorBody(String code,
          {String message = 'Erreur de test.',
          Map<String, dynamic>? details}) =>
      {
        'error': {
          'code': code,
          'message': message,
          if (details != null) 'details': details
        },
      };

  /// Requêtes reçues sur une route.
  List<RecordedRequest> calls(String method, String path) {
    final pattern = _compile(path);
    return [
      for (final r in requests)
        if (r.method == method && pattern.hasMatch(r.path)) r,
    ];
  }

  RecordedRequest? last(String method, String path) {
    final list = calls(method, path);
    return list.isEmpty ? null : list.last;
  }

  FixtureRoute? routeFor(String method, String path) {
    for (final r in routes) {
      if (r.matches(method, path)) return r;
    }
    return null;
  }

  Future<http.Response> _handle(http.Request request) async {
    final body = request.body.isEmpty ? null : jsonDecode(request.body);
    final rec = RecordedRequest(request, body);
    requests.add(rec);

    for (final entry in _overrides.entries) {
      if (entry.key.$1 == rec.method &&
          _compile(entry.key.$2).hasMatch(rec.path)) {
        final (status, json) = entry.value(rec);
        return _json(status, json);
      }
    }

    final route = routeFor(rec.method, rec.path);
    if (route == null) {
      return _json(
          404,
          errorBody('not_found',
              message: 'Route inconnue : ${rec.method} ${rec.path}'));
    }
    final file = route.response;
    return _json(route.status,
        file == null ? null : readFixture(file, fixturesDir: fixturesDir));
  }

  static http.Response _json(int status, Object? body) => http.Response.bytes(
        body == null ? const [] : utf8.encode(jsonEncode(body)),
        status,
        headers: {'content-type': 'application/json; charset=utf-8'},
      );
}
