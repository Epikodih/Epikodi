import 'dart:async';

import 'package:clock/clock.dart';
import 'package:epikodi_client/epikodi_client.dart';
import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

import '../accounts/accounts_flow.dart';
import '../api/api.dart';
import '../config.dart';
import '../l10n/app_localizations.dart';
import 'pairing_panel.dart';
import 'tv_events.dart';

/// Écran de premier lancement : la TV se connecte au serveur (connect),
/// affiche le code d'appairage + QR, et demande un nouveau code via refresh.
/// Dès que le serveur annonce l'appairage (flux temps réel), passe à l'écran des comptes.
/// Tout le visuel vit dans le design system (PairingPanel, GloubiLogo…).
/// Crée l'écoute du flux temps réel (remplaçable dans les tests).
typedef TvEventsFactory = TvEvents Function({
  required String sessionId,
  required void Function(Map<String, dynamic> data) onEvent,
  void Function()? onSessionLost,
});

class PairingScreen extends StatefulWidget {
  const PairingScreen({super.key, this.createClient, this.createEvents});

  /// Par défaut : client HTTP vers [serverUrl]. Les tests y mettent un faux.
  final EpikodiClient Function()? createClient;

  /// Par défaut : [TvEvents] vers [serverUrl]. Les tests y mettent un faux.
  final TvEventsFactory? createEvents;

  @override
  State<PairingScreen> createState() => _PairingScreenState();
}

class _PairingScreenState extends State<PairingScreen> {
  late final _client =
      widget.createClient?.call() ?? EpikodiClient(baseUrl: serverUrl);

  /// Vrai quand le client est passé à l'écran des comptes : c'est lui qui
  /// le ferme (au retour à l'appairage), pas ce dispose().
  bool _handedOff = false;

  Timer? _ticker;
  bool _loading = true;
  String? _error;
  String? _sessionId; // null tant que la TV n'a pas fait connect
  Pairing? _pairing;
  DateTime? _deadline;
  int _remaining = 0;

  /// Le bouton « Nouveau code » est bloqué jusqu'à cette date.
  DateTime? _cooldownUntil;

  /// Au démarrage : fin du « bonjour » de Gloubi. Le code n'apparaît pas
  /// avant, pour qu'on voie l'animation en entier (null ensuite).
  DateTime? _greetingUntil = clock.now().add(Gloubi.greetingDuration);

  /// Flux temps réel du serveur : prévient la TV quand le téléphone a appairé.
  TvEvents? _events;
  String? _eventsSession;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _ticker?.cancel();
    _events?.close();
    // Ne pas fermer le client s'il sert maintenant à l'écran des comptes.
    if (!_handedOff) _client.close();
    super.dispose();
  }

  /// Secondes restantes avant de pouvoir redemander un code (0 = disponible).
  int get _cooldownLeft {
    final until = _cooldownUntil;
    if (until == null) return 0;
    final ms = until.difference(clock.now()).inMilliseconds;
    return ms <= 0 ? 0 : (ms / 1000).ceil();
  }

  /// Clic sur « Nouveau code » : ignoré pendant le délai d'attente.
  void _requestNewCode() {
    if (_cooldownLeft > 0) return;
    _cooldownUntil = clock.now().add(newCodeCooldown);
    _load();
  }

  Future<Pairing> _connectNew() async {
    final res = await _client.tv.connect(
      model: deviceModel,
      platform: tvPlatform,
      appVersion: appVersion,
    );
    _sessionId = res.tvSessionId;
    return res.pairing;
  }

  /// Premier appel : connect. Ensuite : refresh (avec repli sur connect si le
  /// serveur a oublié la session, par exemple après un redémarrage).
  Future<void> _load() async {
    _ticker?.cancel();
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      Pairing pairing;
      if (_sessionId == null) {
        pairing = await _connectNew();
      } else {
        try {
          pairing = await _client.tv.refreshPairing(_sessionId!);
        } on ApiException catch (e) {
          if (e.code != 'tv_session_not_found') rethrow;
          pairing = await _connectNew();
        }
      }
      if (!mounted) return;
      // On part de expires_in_s (durée) et non de expires_at (horloge TV ≠ serveur).
      _deadline = clock.now().add(Duration(seconds: pairing.expiresInS));
      await _waitForGreeting();
      if (!mounted) return;
      setState(() {
        _pairing = pairing;
        _remaining = _deadline!.difference(clock.now()).inSeconds;
        _loading = false;
      });
      _ticker = Timer.periodic(const Duration(seconds: 1), (_) => _tick());
      _listen();
    } on ApiException catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.message;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = AppLocalizations.of(context)!.unexpectedError('$e');
        _loading = false;
      });
    }
  }

  /// Au premier chargement, laisse Gloubi finir son « bonjour » (arrivée +
  /// coucou). Rien à attendre si les animations sont réduites.
  Future<void> _waitForGreeting() async {
    final until = _greetingUntil;
    _greetingUntil = null;
    if (until == null || MediaQuery.disableAnimationsOf(context)) return;
    final left = until.difference(clock.now());
    if (left > Duration.zero) await Future<void>.delayed(left);
  }

  /// S'abonne au flux du serveur (une seule fois par session TV).
  void _listen() {
    final sid = _sessionId;
    if (sid == null || (_events != null && _eventsSession == sid)) return;
    _events?.close();
    _eventsSession = sid;
    void onSessionLost() {
      // Le serveur a oublié la session : nouveau connect, puis on se réabonne.
      _events = null;
      _sessionId = null;
      if (mounted) _load();
    }

    final create = widget.createEvents;
    _events = (create != null
        ? create(
            sessionId: sid, onEvent: _onEvent, onSessionLost: onSessionLost)
        : TvEvents(
            baseUrl: serverUrl,
            sessionId: sid,
            onEvent: _onEvent,
            onSessionLost: onSessionLost,
          ))
      ..start();
  }

  void _onEvent(Map<String, dynamic> data) {
    if (data['phase'] == 'pairing' || !mounted || _handedOff) return;
    _ticker?.cancel();
    _events?.close();
    _events = null;

    // Le client (donc le token de la TV) est transmis à l'écran des comptes.
    final client = _client;
    _handedOff = true;

    Navigator.of(context).pushReplacement(
      MaterialPageRoute(
        builder: (ctx) => AccountsFlow(
          api: Api(client.http),
          data: data,
          // Retour à l'appairage : on ferme ce client, la nouvelle
          // PairingScreen en crée un autre (nouveau connect).
          onBackToPairing: () {
            client.close();
            Navigator.of(ctx).pushReplacement(
              MaterialPageRoute(
                builder: (_) => PairingScreen(
                  createClient: widget.createClient,
                  createEvents: widget.createEvents,
                ),
              ),
            );
          },
        ),
      ),
    );
  }

  void _tick() {
    final left = _deadline!.difference(clock.now()).inSeconds;
    if (left <= 0) {
      _load(); // code expiré : nouveau code (pas soumis au délai du bouton)
      return;
    }
    setState(() => _remaining = left); // met aussi à jour le délai du bouton
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: BrandBackground(
        child: TvSafeArea(
          child: _loading
              ? const _LoadingView()
              : _error != null
                  ? Center(child: _ErrorView(message: _error!, onRetry: _load))
                  : PairingPanel(
                      code: _pairing!.code,
                      qrData: _pairing!.qr.payload,
                      remaining: _remaining,
                      cooldownLeft: _cooldownLeft,
                      onNewCode: _requestNewCode,
                    ),
        ),
      ),
    );
  }
}

class _LoadingView extends StatelessWidget {
  const _LoadingView();

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          // Démarrage : Gloubi sort du sol et dit bonjour, puis attend.
          Gloubi.greeting(size: MediaQuery.textScalerOf(context).scale(240)),
          const SizedBox(height: AppSpacing.lg),
          CircularProgressIndicator(semanticsLabel: l10n.connectingToServer),
        ],
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Pas de connexion ou erreur : Gloubi s'inquiète.
        Gloubi(
          animation: GloubiAnimation.worried,
          size: MediaQuery.textScalerOf(context).scale(200),
        ),
        const SizedBox(height: AppSpacing.md),
        Text(message, style: AppText.title, textAlign: TextAlign.center),
        const SizedBox(height: AppSpacing.lg),
        FocusableCard(
          autofocus: true,
          onTap: onRetry,
          semanticLabel: l10n.retryConnectionLabel,
          padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.xl,
            vertical: AppSpacing.sm,
          ),
          child: Text(l10n.retry, style: AppText.title),
        ),
      ],
    );
  }
}
