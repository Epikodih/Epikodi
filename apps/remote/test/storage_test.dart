import 'dart:math';

import 'package:epikodi_remote/storage/remote_store.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('install_id : UUID v4', () {
    final id = newInstallId(Random(1));
    expect(
        id,
        matches(RegExp(
            r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$')));
    expect(newInstallId(), isNot(newInstallId()));
  });

  test('install_id : toujours le même pour une installation', () async {
    final store = MemoryRemoteStore();
    final first = await store.installId();
    expect(await store.installId(), first);
  });

  test('SavedPairing.isExpired', () {
    SavedPairing p(DateTime? at) => SavedPairing(
        host: 'h', port: 1, token: 't', expiresAt: at, deviceName: 'd');
    final now = DateTime.utc(2026, 11, 18);
    expect(p(null).isExpired(now), isFalse);
    expect(p(now.add(const Duration(days: 1))).isExpired(now), isFalse);
    expect(p(now).isExpired(now), isTrue);
    expect(p(now.subtract(const Duration(seconds: 1))).isExpired(now), isTrue);
  });

  test('MemoryRemoteStore : save, load, clear', () async {
    final store = MemoryRemoteStore();
    expect(await store.load(), isNull);
    const p = SavedPairing(
        host: 'h', port: 1, token: 't', expiresAt: null, deviceName: 'd');
    await store.save(p);
    expect(await store.load(), same(p));
    await store.clear();
    expect(await store.load(), isNull);
  });
}
