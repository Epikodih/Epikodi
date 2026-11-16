import 'package:http/http.dart' as http_pkg;

import 'api_client.dart';
import 'api_exception.dart';
import 'tv_api.dart';

/// Point d'entrée : un seul objet à créer dans l'app.
///
///     final client = EpikodiClient(baseUrl: 'http://10.0.2.2:7878');
///     final res = await client.tv.connect(...);
///
/// [httpClient] permet d'injecter un faux client HTTP dans les tests
/// (voir `package:epikodi_client/testing.dart`).
class EpikodiClient {
  EpikodiClient({required String baseUrl, http_pkg.Client? httpClient})
      : http = ApiClient(baseUrl: baseUrl, httpClient: httpClient) {
    tv = TvApi(http);
  }

  final ApiClient http;
  late final TvApi tv;

  set token(String? value) => http.token = value;

  Future<bool> health() async {
    try {
      final json = await http.get('/api/health');
      return json['status'] == 'ok';
    } on ApiException {
      return false;
    }
  }

  void close() => http.close();
}
