import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_exception.dart';

/// Couche HTTP bas niveau : JSON in/out, token Bearer, erreurs typées.
class ApiClient {
  ApiClient({
    required String baseUrl,
    http.Client? httpClient,
    this.token,
    this.timeout = const Duration(seconds: 10),
  })  : baseUrl = Uri.parse(baseUrl),
        _http = httpClient ?? http.Client();

  final Uri baseUrl;
  final Duration timeout;
  final http.Client _http;

  /// Token Bearer, ajouté à chaque requête s'il est défini.
  String? token;

  Future<Map<String, dynamic>> get(String path, {Map<String, Object?>? query}) =>
      _send('GET', path, query: query);

  Future<Map<String, dynamic>> post(String path, {Object? body}) =>
      _send('POST', path, body: body);

   Future<Map<String, dynamic>> patch(String path, {Object? body}) =>
      _send('PATCH', path, body: body);

  Future<Map<String, dynamic>> _send(
    String method,
    String path, {
    Map<String, Object?>? query,
    Object? body,
  }) async {
    final params = query?.map((k, v) => MapEntry(k, '$v'));
    final uri = baseUrl.replace(
      path: path,
      queryParameters: (params == null || params.isEmpty) ? null : params,
    );

    final request = http.Request(method, uri)
      ..headers['Accept'] = 'application/json';
    if (token != null) request.headers['Authorization'] = 'Bearer $token';
    if (body != null) {
      request.headers['Content-Type'] = 'application/json; charset=utf-8';
      request.body = jsonEncode(body);
    }

    final http.Response response;
    try {
      final streamed = await _http.send(request).timeout(timeout);
      response = await http.Response.fromStream(streamed);
    } on TimeoutException {
      throw const ApiException(
          status: 0, code: 'timeout', message: 'Le serveur ne répond pas.');
    } on http.ClientException catch (e) {
      throw ApiException(
          status: 0, code: 'network_error', message: 'Serveur injoignable : ${e.message}');
    }

    final json = _decode(response);
    if (response.statusCode >= 200 && response.statusCode < 300) return json;

    final err = json['error'];
    if (err is Map<String, dynamic>) {
      throw ApiException(
        status: response.statusCode,
        code: (err['code'] as String?) ?? 'unknown_error',
        message: (err['message'] as String?) ?? 'Erreur inconnue.',
        details: err['details'] as Map<String, dynamic>?,
      );
    }
    throw ApiException(
        status: response.statusCode,
        code: 'unknown_error',
        message: 'Erreur HTTP ${response.statusCode}.');
  }

  Map<String, dynamic> _decode(http.Response r) {
    if (r.bodyBytes.isEmpty) return {};
    try {
      final data = jsonDecode(utf8.decode(r.bodyBytes));
      return data is Map<String, dynamic> ? data : {};
    } on FormatException {
      return {};
    }
  }

  void close() => _http.close();
}
