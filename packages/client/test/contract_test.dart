// Contrat avec le serveur : les fixtures de /fixtures sont la référence.
// Si un champ, une route ou un fichier change là-bas, ces tests échouent.
import 'dart:io';

import 'package:epikodi_client/epikodi_client.dart';
import 'package:epikodi_client/testing.dart';
import 'package:test/test.dart';

/// Vérifie que [actual] a exactement les clés de [expected], avec les mêmes
/// types (récursivement). Les valeurs, elles, peuvent différer.
void expectSameShape(Object? actual, Object? expected, [String path = r'$']) {
  if (expected == null) return; // null dans une fixture : type libre
  if (expected is Map) {
    expect(actual, isA<Map>(), reason: '$path doit être un objet');
    final a = actual as Map;
    expect(a.keys.toSet(), expected.keys.toSet(), reason: 'clés de $path');
    for (final k in expected.keys) {
      expectSameShape(a[k], expected[k], '$path.$k');
    }
  } else if (expected is List) {
    expect(actual, isA<List>(), reason: '$path doit être une liste');
    if (expected.isNotEmpty) {
      for (final item in actual as List) {
        expectSameShape(item, expected.first, '$path[]');
      }
    }
  } else if (expected is num) {
    expect(actual, isA<num>(), reason: '$path doit être un nombre');
  } else {
    expect(actual.runtimeType, expected.runtimeType, reason: 'type de $path');
  }
}

void main() {
  final dir = findFixturesDir();
  final backend = FixtureBackend(fixturesDir: dir);

  group('fixtures/routes.json', () {
    test('chaque fichier cité existe et contient du JSON valide', () {
      for (final r in backend.routes) {
        for (final f in [r.request, r.response]) {
          if (f == null) continue;
          expect(File('${dir.path}/$f').existsSync(), isTrue,
              reason: '${r.method} ${r.path} cite $f, introuvable');
          expect(() => readFixture(f, fixturesDir: dir), returnsNormally);
        }
      }
    });

    test('aucune route en double', () {
      final keys = backend.routes.map((r) => '${r.method} ${r.path}').toList();
      expect(keys.toSet().length, keys.length);
    });

    test('chaque fixture de requête/réponse est utilisée par une route', () {
      final used = {
        for (final r in backend.routes)
          ...[r.request, r.response].whereType<String>(),
        'routes.json',
        'errors/error.json', // modèle d'erreur commun
      };
      final unused = <String>[];
      for (final f in dir.listSync(recursive: true).whereType<File>()) {
        if (!f.path.endsWith('.json')) continue;
        final rel = f.path
            .substring(dir.path.length + 1)
            .replaceAll(Platform.pathSeparator, '/');
        if (!used.contains(rel)) unused.add(rel);
      }
      expect(unused, isEmpty, reason: 'fixtures absentes de routes.json');
    });

    test('les routes de connexion TV sont décrites', () {
      for (final route in [
        'GET /api/health',
        'POST /api/tv/connect',
        'POST /api/tv/pairing/refresh',
        'POST /api/remote/pair',
      ]) {
        final [method, path] = route.split(' ');
        expect(backend.routeFor(method, path), isNotNull, reason: route);
      }
    });

    test('refresh peut répondre tv_session_not_found (repli sur connect)', () {
      final route = backend.routeFor('POST', '/api/tv/pairing/refresh')!;
      expect(route.errorCodes, contains('tv_session_not_found'));
    });

    test('le modèle d\'erreur a la forme {error: {code, message, details}}',
        () {
      final json = readFixture('errors/error.json', fixturesDir: dir) as Map;
      final err = json['error'] as Map;
      expect(err['code'], isA<String>());
      expect(err['message'], isA<String>());
      expect(err['details'], anyOf(isNull, isA<Map>()));
    });
  });

  group('modèles', () {
    test('TvConnectResponse lit tv/connect_response.json', () {
      final json = readFixture('tv/connect_response.json', fixturesDir: dir);
      final res = TvConnectResponse.fromJson(json);
      expect(res.tvSessionId, json['tv_session_id']);
      expect(res.lanIp, json['lan_ip']);
      expect(res.pairing.code, json['pairing']['code']);
      expect(res.pairing.expiresInS, json['pairing']['expires_in_s']);
      expect(res.pairing.attemptsLeft, json['pairing']['attempts_left']);
      expect(
          res.pairing.expiresAt, DateTime.parse(json['pairing']['expires_at']));
      expect(res.pairing.qr.payload, json['pairing']['qr']['payload']);
      expect(res.pairing.qr.format, json['pairing']['qr']['format']);
    });

    test('Pairing lit tv/refresh_response.json', () {
      final json = readFixture('tv/refresh_response.json', fixturesDir: dir);
      final p = Pairing.fromJson(json['pairing']);
      expect(p.code, json['pairing']['code']);
      expect(p.expiresAt.isUtc, isTrue);
    });

    test('le code d\'appairage fait 6 chiffres', () {
      for (final f in [
        'tv/connect_response.json',
        'tv/refresh_response.json'
      ]) {
        final code =
            readFixture(f, fixturesDir: dir)['pairing']['code'] as String;
        expect(code, matches(RegExp(r'^\d{6}$')), reason: f);
      }
    });

    test('le QR ne contient pas le code d\'appairage', () {
      final json = readFixture('tv/connect_response.json', fixturesDir: dir);
      final payload = json['pairing']['qr']['payload'] as String;
      expect(payload, isNot(contains(json['pairing']['code'])));
      final uri = Uri.parse(payload);
      expect(uri.queryParameters['host'], json['lan_ip']);
      expect(uri.queryParameters['port'], isNotEmpty);
      expect(uri.queryParameters['sid'], json['tv_session_id']);
    });
  });

  group('requêtes envoyées par TvApi', () {
    late FixtureBackend b;
    late EpikodiClient client;

    setUp(() {
      b = FixtureBackend(fixturesDir: dir);
      client = EpikodiClient(baseUrl: 'http://tv.test', httpClient: b.client);
    });

    test('connect envoie la forme de tv/connect_request.json', () async {
      await client.tv.connect(
          model: 'Desktop linux', platform: 'linux', appVersion: '9.9.9');
      final sent = b.last('POST', '/api/tv/connect')!;
      expectSameShape(
          sent.body, readFixture('tv/connect_request.json', fixturesDir: dir));
      expect(sent.json['platform'], 'linux');
    });

    test('refresh envoie la forme de tv/refresh_request.json', () async {
      await client.tv.refreshPairing('abc');
      final sent = b.last('POST', '/api/tv/pairing/refresh')!;
      expectSameShape(
          sent.body, readFixture('tv/refresh_request.json', fixturesDir: dir));
      expect(sent.json['tv_session_id'], 'abc');
    });
  });
}
