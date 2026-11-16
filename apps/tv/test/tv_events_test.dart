// TvEvents contre un vrai petit serveur HTTP local (Server-Sent Events).
// Pas de testWidgets ici : le réseau réel reste autorisé.
import 'dart:async';
import 'dart:io';

import 'package:epikodi_tv/pairing/tv_events.dart';
import 'package:flutter_test/flutter_test.dart';

/// Serveur SSE minimal : chaque connexion est confiée à [onConnection].
class SseServer {
  SseServer._(this._server);

  final HttpServer _server;
  final requests = <HttpRequest>[];
  final _connections = StreamController<HttpRequest>.broadcast();
  void Function(HttpRequest req)? onConnection;

  static Future<SseServer> start() async {
    final s =
        SseServer._(await HttpServer.bind(InternetAddress.loopbackIPv4, 0));
    s._server.listen((req) {
      s.requests.add(req);
      s._connections.add(req);
      s.onConnection?.call(req);
    });
    return s;
  }

  String get baseUrl => 'http://127.0.0.1:${_server.port}';

  Future<HttpRequest> get nextConnection => _connections.stream.first;

  static void open(HttpRequest req) {
    req.response
      ..statusCode = 200
      ..headers.contentType =
          ContentType('text', 'event-stream', charset: 'utf-8')
      ..bufferOutput = false;
  }

  static Future<void> send(HttpRequest req, String json) async {
    req.response.write('data: $json\n\n');
    await req.response.flush();
  }

  Future<void> close() => _server.close(force: true);
}

void main() {
  late SseServer server;
  TvEvents? events;

  setUp(() async => server = await SseServer.start());
  tearDown(() async {
    events?.close();
    await server.close();
  });

  TvEvents listen({
    required void Function(Map<String, dynamic>) onEvent,
    void Function()? onSessionLost,
  }) =>
      events = TvEvents(
        baseUrl: server.baseUrl,
        sessionId: 'a3f9c2e1',
        onEvent: onEvent,
        onSessionLost: onSessionLost,
        retryDelay: const Duration(milliseconds: 50),
      )..start();

  test('ouvre GET /api/tv/events avec la session et Accept: text/event-stream',
      () async {
    server.onConnection = SseServer.open;
    listen(onEvent: (_) {});
    final req = await server.nextConnection;
    expect(req.method, 'GET');
    expect(req.uri.path, '/api/tv/events');
    expect(req.uri.queryParameters['tv_session_id'], 'a3f9c2e1');
    expect(req.headers.value(HttpHeaders.acceptHeader), 'text/event-stream');
  });

  test('transmet chaque ligne data: décodée, dans l\'ordre', () async {
    final got = <Map<String, dynamic>>[];
    final done = Completer<void>();
    server.onConnection = (req) async {
      SseServer.open(req);
      await SseServer.send(req, '{"phase":"pairing"}');
      req.response.write(': commentaire ignoré\nevent: state\n');
      await SseServer.send(
          req, '{"phase":"paired","profiles":[{"id":1,"name":"Léa"}]}');
    };
    listen(onEvent: (e) {
      got.add(e);
      if (got.length == 2) done.complete();
    });
    await done.future.timeout(const Duration(seconds: 5));
    expect(got[0], {'phase': 'pairing'});
    expect(got[1]['phase'], 'paired');
    expect(got[1]['profiles'][0]['name'], 'Léa');
  });

  test(
      'ignore un JSON illisible ou qui n\'est pas un objet sans couper le flux',
      () async {
    final got = <Map<String, dynamic>>[];
    final done = Completer<void>();
    server.onConnection = (req) async {
      SseServer.open(req);
      await SseServer.send(req, '{pas du json');
      await SseServer.send(req, '[1, 2]');
      await SseServer.send(req, '{"phase":"ready"}');
    };
    listen(onEvent: (e) {
      got.add(e);
      done.complete();
    });
    await done.future.timeout(const Duration(seconds: 5));
    expect(got, [
      {'phase': 'ready'},
    ]);
    expect(server.requests, hasLength(1), reason: 'pas de reconnexion');
  });

  test('404 : session oubliée -> onSessionLost, et plus de tentative',
      () async {
    server.onConnection = (req) {
      req.response
        ..statusCode = 404
        ..close();
    };
    final lost = Completer<void>();
    listen(
        onEvent: (_) => fail('aucun événement attendu'),
        onSessionLost: lost.complete);
    await lost.future.timeout(const Duration(seconds: 5));
    await Future<void>.delayed(const Duration(milliseconds: 200));
    expect(server.requests, hasLength(1));
  });

  test('se reconnecte quand le serveur coupe la connexion', () async {
    var count = 0;
    final second = Completer<void>();
    server.onConnection = (req) async {
      count++;
      SseServer.open(req);
      await SseServer.send(req, '{"phase":"pairing","n":$count}');
      if (count == 1) await req.response.close(); // coupure
    };
    final got = <int>[];
    listen(onEvent: (e) {
      got.add(e['n'] as int);
      if (got.length == 2) second.complete();
    });
    await second.future.timeout(const Duration(seconds: 5));
    expect(got, [1, 2]);
  });

  test('réessaie tant que le serveur est injoignable', () async {
    // Port fermé : on libère un port puis on écoute dessus plus tard.
    final tmp = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    final port = tmp.port;
    await tmp.close(force: true);

    final got = Completer<Map<String, dynamic>>();
    events = TvEvents(
      baseUrl: 'http://127.0.0.1:$port',
      sessionId: 's',
      onEvent: got.complete,
      retryDelay: const Duration(milliseconds: 50),
    )..start();

    await Future<void>.delayed(const Duration(milliseconds: 150));
    final late = await HttpServer.bind(InternetAddress.loopbackIPv4, port);
    late.listen((req) async {
      SseServer.open(req);
      await SseServer.send(req, '{"phase":"pairing"}');
    });
    expect(await got.future.timeout(const Duration(seconds: 5)),
        {'phase': 'pairing'});
    events!.close();
    await late.close(force: true);
  });

  test('close() arrête l\'écoute : plus d\'événement ni de reconnexion',
      () async {
    final conn = Completer<HttpRequest>();
    server.onConnection = (req) {
      SseServer.open(req);
      conn.complete(req);
    };
    var received = 0;
    listen(onEvent: (_) => received++);
    final req = await conn.future.timeout(const Duration(seconds: 5));
    events!.close();
    try {
      await SseServer.send(req, '{"phase":"paired"}');
    } catch (_) {/* la connexion peut déjà être fermée */}
    await Future<void>.delayed(const Duration(milliseconds: 200));
    expect(received, 0);
    expect(server.requests, hasLength(1));
  });
}
