// Outils communs aux tests de la télécommande.
import 'package:epikodi_client/epikodi_client.dart';
import 'package:epikodi_client/testing.dart';
import 'package:epikodi_remote/app.dart';
import 'package:epikodi_remote/deps.dart';
import 'package:epikodi_remote/l10n/app_localizations.dart';
import 'package:epikodi_remote/locale.dart';
import 'package:epikodi_remote/remote/remote_events.dart';
import 'package:epikodi_remote/remote/tv_commands.dart';
import 'package:epikodi_remote/storage/remote_store.dart';
import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

export 'package:epikodi_client/testing.dart';

/// Textes français de l'app (langue des tests).
final fr = lookupAppLocalizations(const Locale('fr'));

/// Identifiant d'installation de la fixture `pairing/request.json`.
const fixtureInstallId = '7c1f3a52-8b4e-4d0a-9e61-2f5b8c3d1a90';

/// QR de la fixture `tv/connect_response.json`.
final fixtureQr = (readFixture('tv/connect_response.json')
    as Map)['pairing']['qr']['payload'] as String;

/// TV appairée, telle que l'app la garde.
SavedPairing savedPairing({DateTime? expiresAt}) => SavedPairing(
      host: '192.168.1.42',
      port: 7878,
      token: 'mc_9fK2xQ7LrT3vB8nW5yZ1aD6hJ4sE0pUc',
      expiresAt: expiresAt ?? DateTime.utc(2100),
      deviceName: 'Pixel 7',
    );

/// Faux monde extérieur : serveur à fixtures, stockage en mémoire,
/// scanner qui renvoie [scanResult].
class TestWorld {
  /// [saved] : déjà appairé dans cette session (l'app démarre sur la
  /// télécommande). [connected] faux : [saved] n'est qu'un reste d'un
  /// lancement précédent, gardé dans le stockage.
  TestWorld({
    SavedPairing? saved,
    String? server,
    this.scanResult,
    bool connected = true,
  })  : initialPairing = connected ? saved : null,
        store = MemoryRemoteStore(
            saved: saved, server: server, installId: fixtureInstallId);

  final SavedPairing? initialPairing;

  /// Adresse du serveur « en dur » dans la config, pour les tests.
  static const defaultServer = '192.168.1.42:7878';

  final backend = FixtureBackend();
  final MemoryRemoteStore store;

  /// Ce que « lit » la caméra (`null` : l'utilisateur revient en arrière).
  String? scanResult;
  int scans = 0;

  /// Touches et raccourcis envoyés à la TV.
  final commands = FakeTvCommands();

  /// Événements que le test pousse au téléphone (comme le serveur).
  final events = FakeRemoteEvents();

  /// Vrai : le serveur est injoignable (erreur réseau).
  bool offline = false;

  /// Adresses vers lesquelles l'app a créé un client.
  final baseUrls = <String>[];

  late final http.Client _offline = MockClient(
      (_) async => throw http.ClientException('Connection refused'));

  RemoteDeps get deps => RemoteDeps(
        store: store,
        deviceName: () async => 'Pixel 7',
        createClient: (url) {
          baseUrls.add(url);
          return EpikodiClient(
            baseUrl: url,
            httpClient: _Switch(this),
          );
        },
        defaultServer: defaultServer,
        commands: (_) => commands,
        events: (_, onEvent) => events..onEvent = onEvent,
        scanQr: (_) async {
          scans++;
          return scanResult;
        },
      );
}

/// Client HTTP qui suit `world.offline` à chaque requête.
class _Switch extends http.BaseClient {
  _Switch(this.world);

  final TestWorld world;

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) =>
      (world.offline ? world._offline : world.backend.client).send(request);

  // Le client de test est partagé entre les écrans : on ne le ferme pas.
  @override
  void close() {}
}

/// Monte l'app sur un écran de téléphone, animations réduites par défaut
/// (Gloubi affiche une image fixe ; voir [gloubiShowing]).
Future<void> pumpRemote(
  WidgetTester tester,
  TestWorld world, {
  bool reducedMotion = true,
  Locale locale = const Locale('fr'),
  Size size = const Size(412, 915),
}) async {
  tester.view
    ..physicalSize = size
    ..devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  tester.platformDispatcher.accessibilityFeaturesTestValue =
      FakeAccessibilityFeatures(disableAnimations: reducedMotion);
  addTearDown(tester.platformDispatcher.clearAccessibilityFeaturesTestValue);
  remoteLocale.value = null;
  addTearDown(() => remoteLocale.value = null);

  await tester.pumpWidget(GloubiRemoteApp(
    deps: world.deps,
    locale: locale,
    initialPairing: world.initialPairing,
  ));
  await settle(tester);
}

/// Laisse passer les réponses du faux serveur (microtâches) et les rebuilds.
Future<void> settle(WidgetTester tester) async {
  for (var i = 0; i < 6; i++) {
    await tester.pump(const Duration(milliseconds: 20));
  }
}

/// Démonte l'arbre : annule les timers (Gloubi, compte à rebours).
Future<void> unmount(WidgetTester tester) async {
  await tester.pumpWidget(const SizedBox());
  await tester.pump(const Duration(seconds: 10));
}

/// Tape [code] dans la saisie du code (le champ invisible sous les tuiles).
Future<void> typeCode(WidgetTester tester, String code) async {
  // Comme l'utilisateur : toucher les tuiles ouvre le clavier.
  await tester.tap(find.byKey(const ValueKey('code-field')), warnIfMissed: false);
  await tester.pump();
  await tester.enterText(find.byKey(const ValueKey('code-field')), code);
  await settle(tester);
}

/// Bouton (BrandButton / GhostButton) par son libellé.
Finder button(String label) => find.byWidgetPredicate(
      (w) => w is Semantics && w.properties.label == label &&
          (w.properties.button ?? false),
      description: 'bouton "$label"',
    );

/// Vrai si le bouton [label] est actif.
bool isEnabled(WidgetTester tester, String label) =>
    tester.widget<Semantics>(button(label)).properties.enabled ?? false;

/// Une image de Gloubi affichant [file] (ex. `worried.png`, `wave.webp`).
Finder gloubiShowing(String file) => find.byWidgetPredicate(
      (w) =>
          w is Image &&
          w.image is AssetImage &&
          (w.image as AssetImage).assetName == 'assets/mascot/$file',
      description: 'Gloubi($file)',
    );

/// Garde les touches et raccourcis envoyés, dans l'ordre.
class FakeTvCommands implements TvCommands {
  final sent = <Object>[];

  @override
  Future<void> key(TvKey key) async => sent.add(key);

  @override
  Future<void> shortcut(TvShortcut shortcut) async => sent.add(shortcut);

  /// Textes envoyés : `('Léa', false)`, `('Léa', true)` si validé.
  final texts = <(String, bool)>[];

  @override
  Future<void> text(String value, {bool done = false}) async =>
      texts.add((value, done));

  /// Langues envoyées à la TV.
  final languages = <String>[];

  @override
  Future<void> language(String code) async => languages.add(code);

  int unpairs = 0;

  @override
  Future<void> unpair() async => unpairs++;
}

/// Faux flux du serveur : le test appelle [emit].
class FakeRemoteEvents implements RemoteEventsSource {
  void Function(Map<String, dynamic>)? onEvent;
  bool started = false;
  bool closed = false;

  @override
  void start() => started = true;

  @override
  void close() => closed = true;

  void emit(Map<String, dynamic> event) => onEvent?.call(event);
}
