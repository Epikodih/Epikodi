// Envoi des touches : routes provisoires du faux serveur (test_api.py).
import 'package:epikodi_client/epikodi_client.dart';
import 'package:epikodi_remote/remote/tv_commands.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/harness.dart';

void main() {
  late FixtureBackend backend;
  late EpikodiClient client;

  setUp(() {
    backend = FixtureBackend();
    backend.on('POST', '/api/remote/key', (_) => (200, {'ok': true}));
    backend.on('POST', '/api/remote/shortcut', (_) => (200, {'ok': true}));
    client = EpikodiClient(baseUrl: 'http://tv.test', httpClient: backend.client)
      ..token = 'mc_token';
  });

  test('touche : POST /api/remote/key avec le token', () async {
    await HttpTvCommands(client).key(TvKey.up);
    final sent = backend.last('POST', '/api/remote/key')!;
    expect(sent.json, {'key': 'up'});
    expect(sent.headers['Authorization'], 'Bearer mc_token');
  });

  test('raccourci : POST /api/remote/shortcut', () async {
    await HttpTvCommands(client).shortcut(TvShortcut.profile);
    expect(backend.last('POST', '/api/remote/shortcut')!.json,
        {'shortcut': 'profile'});
  });

  test('route absente (vrai backend) : erreur ignorée', () async {
    backend.reset('POST', '/api/remote/key');
    await expectLater(HttpTvCommands(client).key(TvKey.ok), completes);
  });
}
