import 'package:device_info_plus/device_info_plus.dart';
import 'package:epikodi_client/epikodi_client.dart';
import 'package:flutter/widgets.dart';

import 'config.dart';
import 'remote/remote_events.dart';
import 'remote/tv_commands.dart';
import 'storage/remote_store.dart';

/// Ouvre la caméra et renvoie le texte du QR lu (`null` si annulé).
typedef QrScanner = Future<String?> Function(BuildContext context);

/// Ce dont les écrans ont besoin du monde extérieur. Les tests remplacent
/// chaque élément (faux serveur, stockage en mémoire, faux scanner).
class RemoteDeps {
  const RemoteDeps({
    required this.store,
    this.deviceName = readDeviceName,
    this.createClient = _defaultClient,
    this.scanQr,
    this.defaultServer = defaultServerAddress,
    this.commands = _httpCommands,
    this.events = _sseEvents,
  });

  /// Événements du serveur pour le téléphone (ex. champ texte de la TV).
  final RemoteEventsSource Function(
          EpikodiClient client, void Function(Map<String, dynamic>) onEvent)
      events;

  static RemoteEventsSource _sseEvents(EpikodiClient client,
          void Function(Map<String, dynamic>) onEvent) =>
      SseRemoteEvents(
        baseUrl: client.http.baseUrl.toString(),
        token: client.token,
        onEvent: onEvent,
      );

  /// Envoi des touches et raccourcis à la TV (voir [TvCommands]).
  final TvCommands Function(EpikodiClient client) commands;

  static TvCommands _httpCommands(EpikodiClient client) =>
      HttpTvCommands(client);

  /// Adresse du serveur tant que l'utilisateur n'en a pas choisi une autre.
  final String defaultServer;

  final RemoteStore store;

  /// Nom du téléphone envoyé au serveur (ex. « Pixel 7 »).
  final Future<String> Function() deviceName;

  /// Client HTTP vers l'adresse de la TV.
  final EpikodiClient Function(String baseUrl) createClient;

  /// `null` : écran caméra de l'app (voir `ScanScreen`).
  final QrScanner? scanQr;

  static EpikodiClient _defaultClient(String baseUrl) =>
      EpikodiClient(baseUrl: baseUrl);

  static RemoteDeps of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<RemoteDepsScope>();
    assert(scope != null, 'Aucun RemoteDepsScope au-dessus de ce widget.');
    return scope!.deps;
  }
}

class RemoteDepsScope extends InheritedWidget {
  const RemoteDepsScope({super.key, required this.deps, required super.child});

  final RemoteDeps deps;

  @override
  bool updateShouldNotify(RemoteDepsScope old) => old.deps != deps;
}

/// Modèle du téléphone (ex. « Pixel 7 »).
Future<String> readDeviceName() async {
  try {
    final info = await DeviceInfoPlugin().androidInfo;
    final model = info.model.trim();
    return model.isEmpty ? fallbackDeviceName : model;
  } catch (_) {
    return fallbackDeviceName;
  }
}
