import 'api_client.dart';
import 'models/remote_pairing.dart';

/// Routes utilisées par la télécommande (téléphone).
class RemoteApi {
  const RemoteApi(this._client);

  final ApiClient _client;

  /// POST /api/remote/pair : envoie le code affiché par la TV.
  ///
  /// En cas de succès, le token reçu est gardé par le client : les requêtes
  /// suivantes l'envoient automatiquement.
  /// Lève une `ApiException` : `pairing_code_invalid` (`details.attempts_left`),
  /// `pairing_code_expired` ou `pairing_too_many_attempts`
  /// (`details.retry_after_s`).
  Future<PairResponse> pair({
    required String code,
    required RemoteDevice device,
  }) async {
    final json = await _client.post('/api/remote/pair', body: {
      'code': code,
      'device': device.toJson(),
    });
    final res = PairResponse.fromJson(json);
    _client.token = res.token;
    return res;
  }

  /// GET /api/session : le téléphone appairé et le profil actif sur la TV.
  /// Lève `ApiException(code: 'unauthorized')` si le token n'est plus valable.
  Future<RemoteSession> session() async =>
      RemoteSession.fromJson(await _client.get('/api/session'));

  /// GET /api/profiles : comptes de la TV (et si l'invité est possible).
  Future<ProfileList> profiles() async =>
      ProfileList.fromJson(await _client.get('/api/profiles'));

  /// POST /api/session/profile : choisit le compte actif sur la TV.
  ///
  /// - invité : `{"type": "guest"}` ;
  /// - compte : `{"type": "account", "profile_id": ...}`, avec `pin` si le
  ///   compte en demande un.
  ///
  /// Lève une `ApiException` : `pin_required`, `pin_invalid`
  /// (`details.attempts_left`), `pin_locked` (`details.retry_after_s`),
  /// `profile_not_found`.
  Future<SessionProfile> selectProfile({
    int? profileId,
    String? pin,
    bool guest = false,
  }) async {
    assert(guest || profileId != null, 'profileId requis pour un compte');
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
    return SessionProfile.fromJson(json['profile'] as Map<String, dynamic>);
  }
}
