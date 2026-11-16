import 'api_client.dart';
import 'models/pairing.dart';
import 'models/tv_connect.dart';

/// Routes utilisées par l'app TV (/api/tv/...).
class TvApi {
  const TvApi(this._client);

  final ApiClient _client;

  /// POST /api/tv/connect : la TV s'enregistre (une fois, au démarrage)
  /// et reçoit sa session + son premier code d'appairage + QR.
  Future<TvConnectResponse> connect({
    required String model,
    required String platform,
    required String appVersion,
  }) async {
    final json = await _client.post('/api/tv/connect', body: {
      'model': model,
      'platform': platform,
      'app_version': appVersion,
    });
    return TvConnectResponse.fromJson(json);
  }

  /// POST /api/tv/pairing/refresh : nouveau code + QR pour une session existante.
  /// Lève ApiException(code: 'tv_session_not_found') si le serveur ne connaît
  /// plus la session (il faut alors rappeler [connect]).
  Future<Pairing> refreshPairing(String tvSessionId) async {
    final json = await _client.post(
      '/api/tv/pairing/refresh',
      body: {'tv_session_id': tvSessionId},
    );
    return Pairing.fromJson(json['pairing'] as Map<String, dynamic>);
  }
}
