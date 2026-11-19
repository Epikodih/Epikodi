import 'dart:math';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// TV appairée, gardée entre deux lancements de l'app.
class SavedPairing {
  const SavedPairing({
    required this.host,
    required this.port,
    required this.token,
    required this.expiresAt,
    required this.deviceName,
  });

  final String host;
  final int port;

  /// Token Bearer reçu à l'appairage.
  final String token;

  /// `null` si le token n'expire pas.
  final DateTime? expiresAt;

  /// Nom du téléphone tel que le serveur l'a enregistré.
  final String deviceName;

  bool isExpired(DateTime now) => expiresAt != null && !now.isBefore(expiresAt!);
}

/// Où l'app garde la TV appairée et l'identifiant d'installation.
abstract class RemoteStore {
  Future<SavedPairing?> load();
  Future<void> save(SavedPairing pairing);
  Future<void> clear();

  /// Identifiant de cette installation (`device.install_id`), créé au premier
  /// appel puis toujours le même.
  Future<String> installId();

  /// Adresse du serveur choisie par l'utilisateur (`ip` ou `ip:port`),
  /// `null` pour celle de la config. Gardée même après « Déconnecter ».
  Future<String?> serverAddress();
  Future<void> setServerAddress(String? address);
}

/// UUID v4 aléatoire.
String newInstallId([Random? random]) {
  final r = random ?? Random.secure();
  final b = List<int>.generate(16, (_) => r.nextInt(256));
  b[6] = (b[6] & 0x0f) | 0x40;
  b[8] = (b[8] & 0x3f) | 0x80;
  final h = b.map((x) => x.toRadixString(16).padLeft(2, '0')).join();
  return '${h.substring(0, 8)}-${h.substring(8, 12)}-${h.substring(12, 16)}-'
      '${h.substring(16, 20)}-${h.substring(20)}';
}

/// Stockage du téléphone : adresse de la TV dans les préférences, token dans
/// le stockage chiffré d'Android (Keystore).
class DeviceRemoteStore implements RemoteStore {
  DeviceRemoteStore({FlutterSecureStorage? secure})
      : _secure = secure ?? const FlutterSecureStorage();

  final FlutterSecureStorage _secure;

  static const _kHost = 'tv.host';
  static const _kPort = 'tv.port';
  static const _kExpires = 'tv.expires_at';
  static const _kDevice = 'tv.device_name';
  static const _kInstall = 'install_id';
  static const _kToken = 'tv.token';
  static const _kServer = 'server.address';

  @override
  Future<SavedPairing?> load() async {
    final prefs = SharedPreferencesAsync();
    final host = await prefs.getString(_kHost);
    final port = await prefs.getInt(_kPort);
    final token = await _secure.read(key: _kToken);
    if (host == null || port == null || token == null) return null;
    final expires = await prefs.getString(_kExpires);
    return SavedPairing(
      host: host,
      port: port,
      token: token,
      expiresAt: expires == null ? null : DateTime.tryParse(expires),
      deviceName: await prefs.getString(_kDevice) ?? '',
    );
  }

  @override
  Future<void> save(SavedPairing p) async {
    final prefs = SharedPreferencesAsync();
    await _secure.write(key: _kToken, value: p.token);
    await prefs.setString(_kHost, p.host);
    await prefs.setInt(_kPort, p.port);
    await prefs.setString(_kDevice, p.deviceName);
    if (p.expiresAt == null) {
      await prefs.remove(_kExpires);
    } else {
      await prefs.setString(_kExpires, p.expiresAt!.toUtc().toIso8601String());
    }
  }

  @override
  Future<void> clear() async {
    final prefs = SharedPreferencesAsync();
    await _secure.delete(key: _kToken);
    for (final k in [_kHost, _kPort, _kExpires, _kDevice]) {
      await prefs.remove(k);
    }
  }

  @override
  Future<String> installId() async {
    final prefs = SharedPreferencesAsync();
    final existing = await prefs.getString(_kInstall);
    if (existing != null) return existing;
    final id = newInstallId();
    await prefs.setString(_kInstall, id);
    return id;
  }

  @override
  Future<String?> serverAddress() => SharedPreferencesAsync().getString(_kServer);

  @override
  Future<void> setServerAddress(String? address) async {
    final prefs = SharedPreferencesAsync();
    if (address == null) {
      await prefs.remove(_kServer);
    } else {
      await prefs.setString(_kServer, address);
    }
  }
}

/// Stockage en mémoire (tests, ou si le stockage du téléphone échoue).
class MemoryRemoteStore implements RemoteStore {
  MemoryRemoteStore({this.saved, this.server, String? installId})
      : _installId = installId;

  SavedPairing? saved;

  /// Adresse du serveur choisie (null : celle de la config).
  String? server;
  String? _installId;

  @override
  Future<SavedPairing?> load() async => saved;

  @override
  Future<void> save(SavedPairing pairing) async => saved = pairing;

  @override
  Future<void> clear() async => saved = null;

  @override
  Future<String> installId() async => _installId ??= newInstallId();

  @override
  Future<String?> serverAddress() async => server;

  @override
  Future<void> setServerAddress(String? address) async => server = address;
}
