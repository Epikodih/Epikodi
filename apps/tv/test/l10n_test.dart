import 'dart:convert';
import 'dart:io';

import 'package:epikodi_tv/l10n/app_localizations.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Map<String, dynamic> arb(String lang) =>
    jsonDecode(File('lib/l10n/app_$lang.arb').readAsStringSync())
        as Map<String, dynamic>;

Set<String> keys(Map<String, dynamic> a) =>
    a.keys.where((k) => !k.startsWith('@')).toSet();

Set<String> placeholders(String text) =>
    RegExp(r'\{(\w+)\}').allMatches(text).map((m) => m.group(1)!).toSet();

void main() {
  final frArb = arb('fr');
  final enArb = arb('en');

  test('fr et en ont exactement les mêmes clés', () {
    expect(keys(enArb).difference(keys(frArb)), isEmpty,
        reason: 'clés en trop dans en');
    expect(keys(frArb).difference(keys(enArb)), isEmpty,
        reason: 'clés manquantes dans en');
  });

  test('mêmes paramètres {x} dans chaque traduction', () {
    for (final k in keys(frArb)) {
      expect(placeholders('${enArb[k]}'), placeholders('${frArb[k]}'),
          reason: k);
    }
  });

  test('aucune traduction vide', () {
    for (final a in [frArb, enArb]) {
      for (final k in keys(a)) {
        expect('${a[k]}'.trim(), isNotEmpty, reason: k);
      }
    }
  });

  test('langues gérées : fr et en', () {
    expect(AppLocalizations.supportedLocales.map((l) => l.languageCode),
        containsAll(['fr', 'en']));
    expect(lookupAppLocalizations(const Locale('en')).whoIsWatching,
        isNot(lookupAppLocalizations(const Locale('fr')).whoIsWatching));
  });
}
