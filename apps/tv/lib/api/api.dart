import 'package:epikodi_client/epikodi_client.dart';

import '../player/playback_state.dart';

/// Routes de comptes / session / médiathèque, utilisées par la TV uniquement.
///
/// Ce fichier vit dans l'app TV. Il ne refait aucun HTTP : tout passe par
/// l'[ApiClient] du package partagé `epikodi_client`, qui gère le JSON,
/// le token Bearer, le timeout et lève des [ApiException] typées
/// (status, code, message).
///
///     final client = EpikodiClient(baseUrl: '...');   // shared
///     final api = Api(client.http);                    // TV
///
/// Le token de la TV (reçu à /api/tv/connect) est celui de l'[ApiClient] :
/// il est envoyé automatiquement à chaque requête.
///
/// Routes :
/// - POST  /api/session/profile -> choisir un compte (PIN) ou l'invité
/// - POST  /api/profiles        -> créer un compte (devient actif)
/// - PATCH /api/profiles/{id}   -> modifier un compte (PIN si restriction d'âge)
/// - GET   /api/media           -> médias du compte actif
/// - GET   /api/media/{id}/stream -> fichier à lire (via [streamSource])
class Api {
  const Api(this._client);

  final ApiClient _client;

  /// Adresse du serveur sans « / » final : sert à construire l'URL des
  /// affiches (`'$baseUrl$poster_url'`).
  String get baseUrl =>
      _client.baseUrl.toString().replaceFirst(RegExp(r'/+$'), '');

  // ---------------------------------------------------------------- comptes

  /// Choisit un compte existant (avec son [pin] si besoin) ou l'invité.
  /// Renvoie le compte actif. Lève [ApiException] (ex. mauvais PIN).
  Future<Map<String, dynamic>> selectProfile({
    Object? profileId,
    String? pin,
    bool guest = false,
  }) async {
    final json = await _client.post(
      '/api/session/profile',
      body: guest
          ? {'type': 'guest'}
          : {
              'type': 'account',
              'profile_id': profileId,
              if (pin != null) 'pin': pin,
            },
    );
    return _activeProfile(json);
  }

  /// Crée un compte. Le serveur le rend actif tout de suite.
  Future<Map<String, dynamic>> createProfile({
    required String name,
    required String icon,
    required String language,
    required int age,
    required bool isRestricted,
  }) async {
    final json = await _client.post('/api/profiles', body: {
      'name': name,
      'icon': icon,
      'language': language,
      'age': age,
      'is_restricted': isRestricted,
    });
    return _activeProfile(json);
  }

  /// Modifie un compte et renvoie le compte à jour.
  /// [icon] : `avatar_0X` (couleur prédéfinie) ou `file:<chemin>` (photo).
  /// [pin] : requis quand on touche à la restriction d'âge. Le serveur doit
  /// le vérifier et répondre avec le code `pin_invalid` s'il est faux
  /// (lève alors une [ApiException]).
  Future<Map<String, dynamic>> updateProfile(
    Object id, {
    required String name,
    required String icon,
    required int age,
    required bool isRestricted,
    required String language,
    String? pin,
  }) async {
    final json = await _client.patch('/api/profiles/$id', body: {
      'name': name,
      'icon': icon,
      'age': age,
      'is_restricted': isRestricted,
      'language': language,
      if (pin != null) 'pin': pin,
    });
    return _activeProfile(json);
  }

  // ------------------------------------------------------------ médiathèque

  /// Médias du compte actif (le serveur filtre déjà selon l'âge).
  /// Le serveur répond { "items": [ ... ], "total", "limit", "offset" }
  /// (voir fixtures/media/list.json).
  Future<List<Map<String, dynamic>>> fetchMedia() async {
    final json = await _client.get('/api/media');
    final list = json['items'] as List? ?? const [];
    return list.whereType<Map<String, dynamic>>().toList();
  }

  // ---------------------------------------------------------------- lecture

  /// Source de lecture d'un média (film, épisode, morceau…).
  ///
  /// Le moteur lit `GET /api/media/{id}/stream` lui-même (requêtes `Range`
  /// pour l'avance rapide) : on lui donne l'URL et le token à mettre dans
  /// l'en-tête `Authorization`, jamais dans l'URL.
  PlaybackSource streamSource(Map<String, dynamic> media) {
    final token = _client.token;
    return PlaybackSource(
      uri: '$baseUrl/api/media/${media['id']}/stream',
      title: '${media['title'] ?? ''}',
      headers: {if (token != null) 'Authorization': 'Bearer $token'},
    );
  }

  // ----------------------------------------------------------------- interne

  /// Réponse { "profile": {...}, "token": "..." } ou le compte directement.
  /// Si le serveur renvoie un nouveau token, on le garde pour la suite.
  Map<String, dynamic> _activeProfile(Map<String, dynamic> json) {
    final token = json['token'];
    if (token is String) _client.token = token;
    final profile = json['profile'];
    return profile is Map<String, dynamic> ? profile : json;
  }
}
