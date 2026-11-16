import 'pairing.dart';

class TvConnectResponse {
  const TvConnectResponse({
    required this.tvSessionId,
    required this.lanIp,
    required this.pairing,
  });

  factory TvConnectResponse.fromJson(Map<String, dynamic> json) =>
      TvConnectResponse(
        tvSessionId: json['tv_session_id'] as String,
        lanIp: json['lan_ip'] as String,
        pairing: Pairing.fromJson(json['pairing'] as Map<String, dynamic>),
      );

  final String tvSessionId;
  final String lanIp;
  final Pairing pairing;
}
