import 'dart:async';
import 'dart:convert';

import 'package:epikodi_client/epikodi_client.dart';
import 'package:epikodi_client/testing.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:test/test.dart';

void main() {
  group('ApiClient', () {
    late FixtureBackend backend;
    late ApiClient api;

    setUp(() {
      backend = FixtureBackend();
      api =
          ApiClient(baseUrl: 'http://tv.test:7878', httpClient: backend.client);
    });

    test('GET : en-tête Accept JSON, pas de Content-Type ni de corps',
        () async {
      await api.get('/api/health');
      final r = backend.last('GET', '/api/health')!;
      expect(r.headers['Accept'], 'application/json');
      expect(r.headers.containsKey('Content-Type'), isFalse);
      expect(r.body, isNull);
      expect(r.raw.url.host, 'tv.test');
      expect(r.raw.url.port, 7878);
    });

    test('POST/PATCH : corps JSON UTF-8', () async {
      await api.post('/api/tv/connect', body: {'model': 'Télé du salon'});
      final r = backend.last('POST', '/api/tv/connect')!;
      expect(r.headers['Content-Type'], startsWith('application/json'));
      expect(r.json['model'], 'Télé du salon');

      await api.patch('/api/profiles/1', body: {'name': 'Léa'});
      expect(backend.last('PATCH', '/api/profiles/1')!.json['name'], 'Léa');
    });

    test('ajoute le token Bearer seulement s\'il est défini', () async {
      await api.get('/api/health');
      expect(
          backend
              .last('GET', '/api/health')!
              .headers
              .containsKey('Authorization'),
          isFalse);

      api.token = 'mc_abc';
      await api.get('/api/health');
      expect(backend.last('GET', '/api/health')!.headers['Authorization'],
          'Bearer mc_abc');
    });

    test('paramètres de requête convertis en texte', () async {
      await api.get('/api/media', query: {'limit': 10, 'kind': 'movie'});
      final r = backend.last('GET', '/api/media')!;
      expect(r.query, {'limit': '10', 'kind': 'movie'});
    });

    test('pas de « ? » vide quand la requête n\'a pas de paramètres', () async {
      await api.get('/api/media', query: {});
      expect(backend.last('GET', '/api/media')!.raw.url.hasQuery, isFalse);
    });

    test('décode les réponses UTF-8 (accents)', () async {
      final json = await api.get('/api/profiles');
      final names = [for (final p in json['profiles']) p['name']];
      expect(names, contains('Léa'));
    });

    test('erreur au format {error} -> ApiException typée', () async {
      backend.fail('POST', '/api/session/profile',
          status: 403,
          code: 'pin_invalid',
          message: 'Code PIN incorrect.',
          details: {'attempts_left': 2});

      await expectLater(
        api.post('/api/session/profile', body: {}),
        throwsA(isA<ApiException>()
            .having((e) => e.status, 'status', 403)
            .having((e) => e.code, 'code', 'pin_invalid')
            .having((e) => e.message, 'message', 'Code PIN incorrect.')
            .having((e) => e.details?['attempts_left'], 'attempts_left', 2)
            .having((e) => e.isNetworkError, 'isNetworkError', isFalse)),
      );
    });

    test('erreur sans corps JSON -> unknown_error avec le statut HTTP',
        () async {
      final client = ApiClient(
        baseUrl: 'http://tv.test',
        httpClient:
            MockClient((_) async => http.Response('<html>oups</html>', 500)),
      );
      await expectLater(
        client.get('/x'),
        throwsA(isA<ApiException>()
            .having((e) => e.status, 'status', 500)
            .having((e) => e.code, 'code', 'unknown_error')),
      );
    });

    test('réponse 2xx vide ou non-objet -> map vide', () async {
      for (final body in ['', '[1,2]', 'pas du json']) {
        final client = ApiClient(
          baseUrl: 'http://tv.test',
          httpClient: MockClient((_) async => http.Response(body, 200)),
        );
        expect(await client.get('/x'), isEmpty, reason: body);
      }
    });

    test('serveur injoignable -> network_error (status 0)', () async {
      final client = ApiClient(
        baseUrl: 'http://tv.test',
        httpClient:
            MockClient((_) async => throw http.ClientException('refused')),
      );
      await expectLater(
        client.get('/x'),
        throwsA(isA<ApiException>()
            .having((e) => e.code, 'code', 'network_error')
            .having((e) => e.isNetworkError, 'isNetworkError', isTrue)),
      );
    });

    test('serveur trop lent -> timeout (status 0)', () async {
      final client = ApiClient(
        baseUrl: 'http://tv.test',
        timeout: const Duration(milliseconds: 20),
        httpClient: MockClient((_) async {
          await Future<void>.delayed(const Duration(milliseconds: 200));
          return http.Response('{}', 200);
        }),
      );
      await expectLater(
        client.get('/x'),
        throwsA(isA<ApiException>()
            .having((e) => e.code, 'code', 'timeout')
            .having((e) => e.status, 'status', 0)),
      );
    });

    test('ApiException.toString est lisible', () {
      const e = ApiException(status: 404, code: 'x', message: 'y');
      expect('$e', 'ApiException(404, x): y');
    });
  });

  group('EpikodiClient', () {
    test('health : true si {status: ok}', () async {
      final backend = FixtureBackend();
      final client =
          EpikodiClient(baseUrl: 'http://tv.test', httpClient: backend.client);
      expect(await client.health(), isTrue);
    });

    test('health : false pendant le démarrage (503 starting)', () async {
      final backend = FixtureBackend()
        ..on('GET', '/api/health', (_) => (503, {'status': 'starting'}));
      final client =
          EpikodiClient(baseUrl: 'http://tv.test', httpClient: backend.client);
      expect(await client.health(), isFalse);
    });

    test('health : false si le serveur est injoignable', () async {
      final client = EpikodiClient(
        baseUrl: 'http://tv.test',
        httpClient:
            MockClient((_) async => throw http.ClientException('refused')),
      );
      expect(await client.health(), isFalse);
    });

    test('token partagé entre client et routes', () async {
      final backend = FixtureBackend();
      final client =
          EpikodiClient(baseUrl: 'http://tv.test', httpClient: backend.client)
            ..token = 'tok';
      await client.tv.refreshPairing('a3f9c2e1');
      expect(
          backend
              .last('POST', '/api/tv/pairing/refresh')!
              .headers['Authorization'],
          'Bearer tok');
    });
  });

  group('TvApi', () {
    late FixtureBackend backend;
    late EpikodiClient client;

    setUp(() {
      backend = FixtureBackend();
      client =
          EpikodiClient(baseUrl: 'http://tv.test', httpClient: backend.client);
    });

    test('connect renvoie session, IP et premier code', () async {
      final res = await client.tv
          .connect(model: 'm', platform: 'windows', appVersion: '0.1.0');
      expect(res.tvSessionId, 'a3f9c2e1');
      expect(res.lanIp, '192.168.1.42');
      expect(res.pairing.code, '482916');
      expect(res.pairing.expiresInS, 300);
    });

    test('refresh renvoie un nouveau code', () async {
      final p = await client.tv.refreshPairing('a3f9c2e1');
      expect(p.code, '735104');
    });

    test('refresh : session oubliée -> tv_session_not_found', () async {
      backend.fail('POST', '/api/tv/pairing/refresh',
          status: 404, code: 'tv_session_not_found');
      await expectLater(
        client.tv.refreshPairing('perdu'),
        throwsA(isA<ApiException>()
            .having((e) => e.code, 'code', 'tv_session_not_found')),
      );
    });

    test('connect : validation_failed remonte en ApiException', () async {
      backend.fail('POST', '/api/tv/connect',
          status: 422,
          code: 'validation_failed',
          details: {
            'fields': ['model']
          });
      await expectLater(
        client.tv.connect(model: '', platform: 'windows', appVersion: '0.1.0'),
        throwsA(isA<ApiException>()
            .having((e) => e.status, 'status', 422)
            .having((e) => e.details?['fields'], 'fields', ['model'])),
      );
    });

    test('réponse connect mal formée -> erreur (pas de valeur inventée)',
        () async {
      backend.on(
          'POST', '/api/tv/connect', (_) => (201, {'tv_session_id': 'x'}));
      await expectLater(
        client.tv.connect(model: 'm', platform: 'windows', appVersion: '0.1.0'),
        throwsA(anything),
      );
    });
  });

  test('FixtureBackend : route inconnue -> 404 not_found', () async {
    final backend = FixtureBackend();
    final res = await backend.client.get(Uri.parse('http://tv.test/api/nope'));
    expect(res.statusCode, 404);
    expect(jsonDecode(res.body)['error']['code'], 'not_found');
  });
}
