// Routes de la télécommande : appairage (POST /api/remote/pair), session,
// et lecture du QR affiché par la TV.
import 'package:epikodi_client/epikodi_client.dart';
import 'package:epikodi_client/testing.dart';
import 'package:test/test.dart';

import 'contract_test.dart' show expectSameShape;

const device = RemoteDevice(
  name: 'Pixel 7',
  installId: '7c1f3a52-8b4e-4d0a-9e61-2f5b8c3d1a90',
  platform: 'android',
);

void main() {
  late FixtureBackend backend;
  late EpikodiClient client;

  setUp(() {
    backend = FixtureBackend();
    client = EpikodiClient(baseUrl: 'http://tv.test', httpClient: backend.client);
  });

  group('PairTarget.fromQr', () {
    test('lit le QR de tv/connect_response.json', () {
      final payload = (readFixture('tv/connect_response.json')
          as Map)['pairing']['qr']['payload'] as String;
      final target = PairTarget.fromQr(payload)!;
      expect(target.host, '192.168.1.42');
      expect(target.port, 7878);
      expect(target.sessionId, 'a3f9c2e1');
      expect(target.baseUrl, 'http://192.168.1.42:7878');
      expect(target.label, '192.168.1.42');
      expect(target.code, '482916', reason: 'le QR remplace le code tapé');
    });

    test('code dans le QR (6 chiffres seulement)', () {
      expect(PairTarget.fromQr('mediacenter://pair?host=1.2.3.4&code=482916')!.code,
          '482916');
      expect(PairTarget.fromQr('mediacenter://pair?host=1.2.3.4&code=12')!.code,
          isNull);
      expect(PairTarget.fromQr('mediacenter://pair?host=1.2.3.4')!.code, isNull);
    });

    test('port absent : port par défaut', () {
      final t = PairTarget.fromQr('mediacenter://pair?host=10.0.0.5')!;
      expect(t.port, defaultServerPort);
      expect(t.sessionId, isNull);
    });

    test('refuse ce qui n\'est pas un QR de TV', () {
      for (final bad in [
        '',
        'https://example.com',
        'mediacenter://other?host=1.2.3.4',
        'mediacenter://pair?port=7878',
        'mediacenter://pair?host=1.2.3.4&port=abc',
        'mediacenter://pair?host=1.2.3.4&port=99999',
      ]) {
        expect(PairTarget.fromQr(bad), isNull, reason: bad);
      }
    });
  });

  group('PairTarget.parse (adresse saisie)', () {
    test('accepte IP, IP:port, URL et QR', () {
      expect(PairTarget.parse('192.168.1.42'),
          const PairTarget(host: '192.168.1.42'));
      expect(PairTarget.parse(' 192.168.1.42:8080 '),
          const PairTarget(host: '192.168.1.42', port: 8080));
      expect(PairTarget.parse('http://tv.local:7878/'),
          const PairTarget(host: 'tv.local'));
      expect(PairTarget.parse('mediacenter://pair?host=1.2.3.4&sid=x'),
          const PairTarget(host: '1.2.3.4', sessionId: 'x'));
    });

    test('label affiche le port seulement s\'il n\'est pas celui par défaut', () {
      expect(PairTarget.parse('10.0.2.2')!.label, '10.0.2.2');
      expect(PairTarget.parse('10.0.2.2:9000')!.label, '10.0.2.2:9000');
    });

    test('refuse une adresse invalide', () {
      for (final bad in [
        '',
        '   ',
        'ftp://1.2.3.4',
        'http://',
        '1.2.3.4/api',
        'pas une adresse',
        'tv_salon',
      ]) {
        expect(PairTarget.parse(bad), isNull, reason: bad);
      }
    });
  });

  group('RemoteApi.pair', () {
    test('envoie la forme de pairing/request.json', () async {
      await client.remote.pair(code: '482916', device: device);
      final sent = backend.last('POST', '/api/remote/pair')!;
      expectSameShape(sent.json, readFixture('pairing/request.json'));
      expect(sent.json, readFixture('pairing/request.json'));
      expect(sent.headers['Authorization'], isNull,
          reason: "pas de token avant l'appairage");
    });

    test('lit pairing/response.json et garde le token', () async {
      final res = await client.remote.pair(code: '482916', device: device);
      expect(res.token, 'mc_9fK2xQ7LrT3vB8nW5yZ1aD6hJ4sE0pUc');
      expect(res.tokenType, 'Bearer');
      expect(res.expiresAt, DateTime.utc(2026, 12, 7, 18, 31, 12));
      expect(res.deviceId, 1);
      expect(res.deviceName, 'Pixel 7');
      expect(client.token, res.token);

      await client.remote.session();
      expect(backend.last('GET', '/api/session')!.headers['Authorization'],
          'Bearer ${res.token}');
    });

    test('expires_at null : token sans expiration', () async {
      backend.on('POST', '/api/remote/pair', (_) {
        final json = Map<String, dynamic>.of(
            readFixture('pairing/response.json') as Map<String, dynamic>);
        json['expires_at'] = null;
        return (200, json);
      });
      final res = await client.remote.pair(code: '482916', device: device);
      expect(res.expiresAt, isNull);
    });

    for (final (status, code) in [
      (403, 'pairing_code_invalid'),
      (410, 'pairing_code_expired'),
      (429, 'pairing_too_many_attempts'),
    ]) {
      test('erreur $code', () async {
        backend.fail('POST', '/api/remote/pair',
            status: status, code: code, details: {'attempts_left': 2});
        await expectLater(
          client.remote.pair(code: '000000', device: device),
          throwsA(isA<ApiException>()
              .having((e) => e.code, 'code', code)
              .having((e) => e.status, 'status', status)
              .having((e) => e.details?['attempts_left'], 'details', 2)),
        );
        expect(client.token, isNull);
      });
    }

    test('les erreurs de la route sont celles de routes.json', () {
      expect(backend.routeFor('POST', '/api/remote/pair')!.errorCodes, [
        'pairing_code_invalid',
        'pairing_code_expired',
        'pairing_too_many_attempts',
      ]);
    });
  });

  group('RemoteApi.session', () {
    test('lit session/current.json', () async {
      final s = await client.remote.session();
      expect(s.deviceId, 1);
      expect(s.deviceName, 'Pixel 7');
      expect(s.pairedAt, DateTime.utc(2026, 11, 7, 18, 31, 12));
      expect(s.profile!.name, 'Léa');
      expect(s.profile!.id, 2);
      expect(s.profile!.isRestricted, isTrue);
      expect(s.profile!.isGuest, isFalse);
    });

    test('profile null tant que la TV n\'a pas choisi de profil', () async {
      backend.on('GET', '/api/session', (_) {
        final json = Map<String, dynamic>.of(
            readFixture('session/current.json') as Map<String, dynamic>);
        json['profile'] = null;
        return (200, json);
      });
      expect((await client.remote.session()).profile, isNull);
    });

    test('invité (id et icône null)', () async {
      backend.on('GET', '/api/session', (_) {
        final json = Map<String, dynamic>.of(
            readFixture('session/current.json') as Map<String, dynamic>);
        json['profile'] = {
          'kind': 'guest',
          'id': null,
          'name': 'Invité',
          'icon': null,
          'language': 'fr',
          'age': 0,
          'is_restricted': true,
        };
        return (200, json);
      });
      final p = (await client.remote.session()).profile!;
      expect(p.isGuest, isTrue);
      expect(p.id, isNull);
    });

    test('token refusé : unauthorized', () async {
      backend.fail('GET', '/api/session', status: 401, code: 'unauthorized');
      await expectLater(
        client.remote.session(),
        throwsA(isA<ApiException>().having((e) => e.code, 'code', 'unauthorized')),
      );
    });
  });

  group('RemoteApi : comptes', () {
    test('profiles lit profiles/list.json', () async {
      final list = await client.remote.profiles();
      expect(list.guestAvailable, isTrue);
      expect(list.canCreate, isTrue);
      expect(list.profiles.map((p) => p.name), ['Marc', 'Léa']);
      expect(list.profiles.first.requiresPin, isTrue);
      expect(list.profiles.last.requiresPin, isFalse);
      expect(list.profiles.last.isRestricted, isTrue);
    });

    test('compte avec PIN : forme de session/select_request.json', () async {
      final p = await client.remote.selectProfile(profileId: 1, pin: '1234');
      final sent = backend.last('POST', '/api/session/profile')!;
      expectSameShape(sent.json, readFixture('session/select_request.json'));
      expect(sent.json, readFixture('session/select_request.json'));
      expect(p.name, 'Marc');
    });

    test('compte sans PIN : profile_id seul', () async {
      await client.remote.selectProfile(profileId: 2);
      expect(backend.last('POST', '/api/session/profile')!.json,
          {'type': 'account', 'profile_id': 2});
    });

    test('invité : type guest seul', () async {
      await client.remote.selectProfile(guest: true);
      expect(backend.last('POST', '/api/session/profile')!.json,
          {'type': 'guest'});
    });

    test('PIN faux : pin_invalid', () async {
      backend.fail('POST', '/api/session/profile',
          status: 403, code: 'pin_invalid', details: {'attempts_left': 2});
      await expectLater(
        client.remote.selectProfile(profileId: 1, pin: '0000'),
        throwsA(isA<ApiException>()
            .having((e) => e.code, 'code', 'pin_invalid')),
      );
    });
  });
}
