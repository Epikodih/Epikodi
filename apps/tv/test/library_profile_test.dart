import 'package:epikodi_tv/api/api.dart';
import 'package:epikodi_tv/library/library_screen.dart';
import 'package:epikodi_tv/profile/profile_screen.dart';
import 'package:epikodi_tv/widgets/profile_menu.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:epikodi_client/epikodi_client.dart';

import 'support/harness.dart';

Map<String, dynamic> get marc => Map<String, dynamic>.of(
    readFixture('session/select_response.json')['profile']);

void main() {
  late FixtureBackend backend;
  setUp(() => backend = FixtureBackend());

  group('Api (routes TV)', () {
    late Api api;
    setUp(() => api = Api(fixtureClient(backend).http));

    test('baseUrl sans « / » final (URL des affiches)', () {
      expect(Api(EpikodiClientFor('http://tv.test:7878/').http).baseUrl,
          'http://tv.test:7878');
    });

    test('fetchMedia lit la clé items de media/list.json', () async {
      final items = await api.fetchMedia();
      expect(items,
          hasLength((readFixture('media/list.json')['items'] as List).length));
      expect(items.first, containsPair('title', isA<String>()));
    });

    test('selectProfile renvoie le profil et garde un nouveau token', () async {
      backend.on(
          'POST',
          '/api/session/profile',
          (_) => (
                200,
                {
                  ...readFixture('session/select_response.json'),
                  'token': 'nouveau'
                }
              ));
      final p = await api.selectProfile(profileId: 1, pin: '1234');
      expect(p['name'], 'Marc');

      await api.fetchMedia();
      expect(backend.last('GET', '/api/media')!.headers['Authorization'],
          'Bearer nouveau');
    });

    test('createProfile renvoie le profil créé (réponse create_response)',
        () async {
      final p = await api.createProfile(
          name: 'Marc',
          icon: 'avatar_03',
          language: 'fr',
          age: 35,
          isRestricted: false);
      expect(p['id'], 1);
      expect(p['kind'], 'account');
    });

    test('updateProfile : PATCH /api/profiles/{id}, forme de update_request',
        () async {
      final p = await api.updateProfile(1,
          name: 'Marc',
          icon: 'avatar_05',
          age: 36,
          isRestricted: false,
          language: 'en',
          pin: '1234');
      final sent = backend.last('PATCH', '/api/profiles/1')!.json;
      final expected =
          readFixture('profiles/update_request.json') as Map<String, dynamic>;
      expect(sent.keys.toSet(), expected.keys.toSet());
      expect(p['icon'], 'avatar_05');
    });

    test('updateProfile sans PIN : pas de clé pin', () async {
      await api.updateProfile(1,
          name: 'M',
          icon: 'avatar_01',
          age: 30,
          isRestricted: false,
          language: 'fr');
      expect(backend.last('PATCH', '/api/profiles/1')!.json.containsKey('pin'),
          isFalse);
    });
  });

  group('LibraryScreen', () {
    Future<List<String>> pumpLibrary(WidgetTester tester,
        {Map<String, dynamic>? profile}) async {
      final calls = <String>[];
      await pumpTv(
        tester,
        LibraryScreen(
          api: Api(fixtureClient(backend).http),
          profile: profile ?? marc,
          onChangeProfile: () => calls.add('switch'),
          onBackToPairing: () => calls.add('pairing'),
        ),
      );
      await settle(tester);
      return calls;
    }

    testWidgets('range les médias par type, dans l\'ordre des rangées',
        (tester) async {
      await pumpLibrary(tester);
      final titles = [
        fr.sectionMovies,
        fr.sectionSeries,
        fr.sectionMusic,
        fr.sectionPodcasts
      ];
      final ys = [for (final t in titles) tester.getTopLeft(find.text(t)).dy];
      expect(ys, orderedEquals([...ys]..sort()));

      for (final t in [
        'Big Buck Bunny',
        'Série Démo',
        'Artiste Démo',
        'Podcast Démo',
        'mon_film_2019_final'
      ]) {
        expect(find.text(t), findsOneWidget, reason: t);
      }
      expect(backend.calls('GET', '/api/media'), hasLength(1));
    });

    testWidgets(
        'affiches : URL du serveur + poster_url, sinon image par défaut',
        (tester) async {
      await pumpLibrary(tester);
      final urls = tester
          .widgetList<Image>(find.byType(Image))
          .map((i) => i.image)
          .whereType<NetworkImage>()
          .map((n) => n.url)
          .toSet();
      expect(urls, contains('$testBaseUrl/api/media/1/poster'));
      expect(urls.where((u) => u.contains('null')), isEmpty,
          reason: 'média sans affiche');
    });

    testWidgets('aucun média : message dédié', (tester) async {
      backend.on('GET', '/api/media',
          (_) => (200, {'items': [], 'total': 0, 'limit': 50, 'offset': 0}));
      await pumpLibrary(tester);
      expect(find.text(fr.noMediaForProfile), findsOneWidget);
    });

    testWidgets('erreur : message du serveur et « Réessayer » recharge',
        (tester) async {
      var n = 0;
      backend.on('GET', '/api/media', (_) {
        n++;
        return n == 1
            ? (
                403,
                FixtureBackend.errorBody('profile_required',
                    message: 'Choisissez un profil.')
              )
            : (200, readFixture('media/list.json'));
      });
      await pumpLibrary(tester);
      expect(find.text('Choisissez un profil.'), findsOneWidget);

      await tester.tap(find.text(fr.retry));
      await settle(tester);
      expect(find.text('Big Buck Bunny'), findsOneWidget);
      expect(n, 2);
    });

    testWidgets('invité : le menu affiche « Invité »', (tester) async {
      await pumpLibrary(tester,
          profile: {'kind': 'guest', 'id': null, 'name': null});
      expect(
          find.descendant(
              of: find.byType(ProfileMenuButton),
              matching: find.text(fr.guestName)),
          findsOneWidget);
    });

    testWidgets('menu profil : « Changer de profil » et « Appairage »',
        (tester) async {
      final calls = await pumpLibrary(tester);
      await tester.tap(find.byType(ProfileMenuButton));
      await tester.pumpAndSettle();
      expect(find.text(fr.profile), findsOneWidget);
      await tester.tap(find.text(fr.switchProfile));
      await tester.pumpAndSettle();

      await tester.tap(find.byType(ProfileMenuButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(fr.pairingButton));
      await tester.pumpAndSettle();
      expect(calls, ['switch', 'pairing']);
    });

    testWidgets(
        'menu profil : « Profil » ouvre l\'écran puis recharge les médias',
        (tester) async {
      await pumpLibrary(tester);
      await tester.tap(find.byType(ProfileMenuButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(fr.profile));
      await tester.pumpAndSettle();
      expect(find.byType(ProfileScreen), findsOneWidget);

      await tester.enterText(find.byType(TextField), 'Marco');
      await tester.tap(find.text(fr.save));
      await settle(tester);
      await tester.pumpAndSettle();
      expect(find.byType(ProfileScreen), findsNothing);
      expect(backend.calls('GET', '/api/media'), hasLength(2));
    });
  });

  group('ProfileScreen', () {
    Future<List<Object?>> pumpProfile(
        WidgetTester tester, Map<String, dynamic> profile) async {
      final popped = <Object?>[];
      await pumpTv(
        tester,
        Builder(
          builder: (context) => TextButton(
            onPressed: () async => popped.add(await Navigator.of(context)
                .push<Object?>(MaterialPageRoute<Object?>(
                    builder: (_) => ProfileScreen(
                        api: Api(fixtureClient(backend).http),
                        profile: profile)))),
            child: const Text('ouvrir'),
          ),
        ),
      );
      await tester.tap(find.text('ouvrir'));
      await tester.pumpAndSettle();
      return popped;
    }

    testWidgets('pré-remplit le nom, l\'âge et la restriction', (tester) async {
      await pumpProfile(tester, marc);
      expect(find.widgetWithText(TextField, 'Marc'), findsOneWidget);
      expect(find.text(fr.ageYears(35)), findsOneWidget);
      expect(find.text(fr.no), findsOneWidget);
    });

    testWidgets('rien de changé : « Enregistrer » ferme sans requête',
        (tester) async {
      final popped = await pumpProfile(tester, marc);
      await tester.tap(find.text(fr.save));
      await tester.pumpAndSettle();
      expect(backend.calls('PATCH', '/api/profiles/{id}'), isEmpty);
      expect(popped, [null]);
    });

    testWidgets('nom changé : PATCH sans PIN, renvoie le profil mis à jour',
        (tester) async {
      final popped = await pumpProfile(tester, marc);
      await tester.enterText(find.byType(TextField), 'Marco');
      await tester.tap(find.text(fr.save));
      await settle(tester);
      await tester.pumpAndSettle();

      final sent = backend.last('PATCH', '/api/profiles/1')!.json;
      expect(sent['name'], 'Marco');
      expect(sent.containsKey('pin'), isFalse);
      expect(popped.single, isA<Map>());
    });

    testWidgets('nom vide : message, aucune requête', (tester) async {
      await pumpProfile(tester, marc);
      await tester.enterText(find.byType(TextField), '');
      await tester.tap(find.text(fr.save));
      await tester.pump();
      expect(find.text(fr.enterName), findsOneWidget);
      expect(backend.calls('PATCH', '/api/profiles/{id}'), isEmpty);
    });

    testWidgets('restriction changée : PIN demandé et envoyé', (tester) async {
      await pumpProfile(tester, marc);
      await tester.tap(find.text(fr.no));
      await tester.pump();
      expect(find.text(fr.pinWillBeAsked), findsOneWidget);
      await tester.tap(find.text(fr.save));
      await tester.pumpAndSettle();
      expect(find.text(fr.pinTitle), findsOneWidget);

      await tapPin(tester, '1234');
      await settle(tester);
      await tester.pumpAndSettle();
      final sent = backend.last('PATCH', '/api/profiles/1')!.json;
      expect(sent['pin'], '1234');
      expect(sent['is_restricted'], isTrue);
    });

    testWidgets(
        'PIN faux (pin_invalid) : redemandé avec « Code PIN incorrect. »',
        (tester) async {
      backend.fail('PATCH', '/api/profiles/{id}',
          status: 403, code: 'pin_invalid');
      await pumpProfile(tester, marc);
      await tester.tap(find.text(fr.no));
      await tester.tap(find.text(fr.save));
      await tester.pumpAndSettle();
      await tapPin(tester, '0000');
      await settle(tester);
      await tester.pumpAndSettle();

      expect(find.text(fr.pinTitle), findsOneWidget);
      expect(find.text(fr.pinWrong), findsOneWidget);

      backend.reset('PATCH', '/api/profiles/{id}');
      await tapPin(tester, '1234');
      await settle(tester);
      await tester.pumpAndSettle();
      expect(find.byType(ProfileScreen), findsNothing);
      expect(backend.calls('PATCH', '/api/profiles/{id}'), hasLength(2));
    });

    testWidgets('PIN annulé : rien n\'est envoyé, on reste sur l\'écran',
        (tester) async {
      await pumpProfile(tester, marc);
      await tester.tap(find.text(fr.no));
      await tester.tap(find.text(fr.save));
      await tester.pumpAndSettle();
      await tester.tap(find.text(fr.cancel));
      await tester.pumpAndSettle();
      expect(backend.calls('PATCH', '/api/profiles/{id}'), isEmpty);
      expect(find.byType(ProfileScreen), findsOneWidget);
    });

    testWidgets('autre erreur serveur : message affiché', (tester) async {
      backend.fail('PATCH', '/api/profiles/{id}',
          status: 409, code: 'profile_name_taken', message: 'Nom déjà pris.');
      await pumpProfile(tester, marc);
      await tester.enterText(find.byType(TextField), 'Léa');
      await tester.tap(find.text(fr.save));
      await settle(tester);
      expect(find.text('Nom déjà pris.'), findsOneWidget);
    });

    testWidgets('fenêtre minimale : pas de débordement', (tester) async {
      await pumpTv(tester,
          ProfileScreen(api: Api(fixtureClient(backend).http), profile: marc),
          size: const Size(960, 540));
      await tester.pump();
      expect(tester.takeException(), isNull);
    });
  });
}

// ignore: non_constant_identifier_names
EpikodiClient EpikodiClientFor(String baseUrl) =>
    EpikodiClient(baseUrl: baseUrl);
