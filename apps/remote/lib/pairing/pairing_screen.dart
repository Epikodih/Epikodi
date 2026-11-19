import 'dart:async';

import 'package:clock/clock.dart';
import 'package:design_system/design_system.dart';
import 'package:epikodi_client/epikodi_client.dart';
import 'package:flutter/material.dart';

import '../config.dart';
import '../connected/connected_screen.dart';
import '../deps.dart';
import '../l10n/app_localizations.dart';
import '../storage/remote_store.dart';
import '../theme.dart';
import '../widgets/buttons.dart';
import '../widgets/code_input.dart';
import '../widgets/remote_page.dart';
import 'scan_screen.dart';

enum _Step {
  /// Connexion au serveur en cours (`GET /api/health`).
  connecting,

  /// Le serveur ne répond pas.
  offline,

  /// Serveur joignable : scanner le QR de la TV ou taper le code.
  pair,

  /// Saisie d'une autre adresse de serveur.
  editServer,
}

/// Connexion à la TV, en deux temps :
///
///  1. **Serveur** : l'adresse du serveur API est en dur
///     ([defaultServerAddress]), ou celle choisie avec « Changer ». L'app
///     vérifie qu'il répond (`GET /api/health`). L'adresse reste affichée en
///     petit en bas, pour pouvoir passer à un autre media center.
///  2. **TV** : scanner le QR affiché sur la TV, ou taper le code à 6
///     chiffres -> `POST /api/remote/pair`. Le QR contient le code
///     (`code=`) : le scan appaire tout seul, c'est une alternative au code
///     tapé. Le serveur reste celui auquel on est déjà connecté. Un QR sans
///     code (ancien serveur) : on tape le code affiché dessous.
///
/// Une fois appairé, le token est gardé et l'app passe à [ConnectedScreen].
class PairingScreen extends StatefulWidget {
  const PairingScreen({super.key, this.sessionExpired = false});

  /// Vrai quand on revient ici parce que la TV a refusé le token.
  final bool sessionExpired;

  @override
  State<PairingScreen> createState() => _PairingScreenState();
}

class _PairingScreenState extends State<PairingScreen> {
  _Step _step = _Step.connecting;

  /// Gloubi dit bonjour en arrivant, une seule fois.
  bool _greet = true;
  late bool _sessionExpired = widget.sessionExpired;

  /// Serveur API. `null` le temps de lire le stockage.
  PairTarget? _server;
  bool _started = false;

  /// Numéro de la dernière tentative de connexion au serveur : une réponse
  /// d'une tentative plus ancienne (serveur changé entre-temps) est ignorée.
  int _attempt = 0;

  final _code = TextEditingController();
  final _codeFocus = FocusNode();
  String? _codeError;
  String? _hint;
  bool _sending = false;

  final _address = TextEditingController();
  String? _addressError;

  /// Après `pairing_too_many_attempts` : pas de nouvel essai avant cette date.
  DateTime? _blockedUntil;
  Timer? _ticker;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!_started) {
      _started = true;
      _start();
    }
  }

  @override
  void dispose() {
    _ticker?.cancel();
    _address.dispose();
    _code.dispose();
    _codeFocus.dispose();
    super.dispose();
  }

  AppLocalizations get _l10n => AppLocalizations.of(context)!;
  RemoteDeps get _deps => RemoteDeps.of(context);

  PairTarget? get _defaultServer => PairTarget.parse(_deps.defaultServer);

  int get _blockedLeft {
    final until = _blockedUntil;
    if (until == null) return 0;
    final ms = until.difference(clock.now()).inMilliseconds;
    return ms <= 0 ? 0 : (ms / 1000).ceil();
  }

  // ---------------------------------------------------------------- serveur

  /// Lit l'adresse choisie (sinon celle de la config), puis s'y connecte.
  Future<void> _start() async {
    final deps = _deps;
    String? saved;
    try {
      saved = await deps.store.serverAddress();
    } catch (_) {
      saved = null;
    }
    if (!mounted) return;
    _server = (saved == null ? null : PairTarget.parse(saved)) ??
        PairTarget.parse(deps.defaultServer);
    await _connect();
  }

  /// Vérifie que le serveur répond (`GET /api/health`).
  Future<void> _connect() async {
    final server = _server;
    final attempt = ++_attempt;
    if (server == null) {
      setState(() => _step = _Step.editServer);
      return;
    }
    setState(() => _step = _Step.connecting);
    final client = _deps.createClient(server.baseUrl);
    final ok = await client.health();
    client.close();
    if (!mounted || attempt != _attempt || _step != _Step.connecting) return;
    setState(() => _step = ok ? _Step.pair : _Step.offline);
  }

  void _openServer() {
    _attempt++; // une connexion en cours ne nous ramènera pas ailleurs
    _address.text = _server?.label ?? '';
    setState(() {
      _greet = false;
      _addressError = null;
      _step = _Step.editServer;
    });
  }

  /// Retour depuis la saisie sans enregistrer : on se reconnecte au serveur
  /// actuel (on ne sait pas si la connexion précédente avait abouti).
  void _closeServer() => _connect();

  /// Garde le nouveau serveur et s'y connecte.
  Future<void> _useServer(PairTarget target) async {
    final def = _defaultServer;
    final isDefault =
        def != null && def.host == target.host && def.port == target.port;
    await _deps.store
        .setServerAddress(isDefault ? null : '${target.host}:${target.port}');
    if (!mounted) return;
    _server = PairTarget(host: target.host, port: target.port);
    _code.clear();
    _codeError = null;
    _hint = null;
    _sessionExpired = false;
    await _connect();
  }

  void _saveAddress() {
    final target = PairTarget.parse(_address.text);
    if (target == null) {
      setState(() => _addressError = _l10n.addressInvalid);
      return;
    }
    _useServer(target);
  }

  // ---------------------------------------------------------------- TV

  Future<void> _scan() async {
    final raw = await (_deps.scanQr ?? openScanScreen)(context);
    if (!mounted || raw == null) return;
    if (raw == ScanScreen.manualEntry) {
      _codeFocus.requestFocus();
      return;
    }
    final qr = PairTarget.fromQr(raw);
    setState(() {
      _greet = false;
      _codeError = qr == null ? _l10n.scanNotTv : null;
      _hint = null;
    });
    if (qr == null) return;
    final code = qr.code;
    if (code != null) {
      _code.text = code;
      await _pair();
    } else {
      // QR sans code (ancien serveur) : on tape le code affiché dessous.
      setState(() => _hint = _l10n.qrNoCode);
      _codeFocus.requestFocus();
    }
  }

  Future<void> _pair() async {
    final server = _server;
    final code = _code.text;
    if (server == null || code.length != 6 || _sending || _blockedLeft > 0) {
      return;
    }
    final deps = _deps;
    setState(() {
      _sending = true;
      _codeError = null;
      _hint = null;
      _sessionExpired = false;
      _greet = false;
    });

    final client = deps.createClient(server.baseUrl);
    var handedOff = false;
    try {
      final device = RemoteDevice(
        name: await deps.deviceName(),
        installId: await deps.store.installId(),
        platform: devicePlatform,
      );
      final res = await client.remote.pair(code: code, device: device);
      final saved = SavedPairing(
        host: server.host,
        port: server.port,
        token: res.token,
        expiresAt: res.expiresAt,
        deviceName: res.deviceName,
      );
      await deps.store.save(saved);
      if (!mounted) return;
      handedOff = true;
      Navigator.of(context).pushReplacement(MaterialPageRoute(
        builder: (_) => ConnectedScreen(
          pairing: saved,
          client: client,
          justPaired: true,
        ),
      ));
    } on ApiException catch (e) {
      if (mounted) _showPairError(e, server);
    } catch (e) {
      if (mounted) setState(() => _codeError = _l10n.errorUnexpected('$e'));
    } finally {
      if (!handedOff) client.close();
      if (mounted && !handedOff) setState(() => _sending = false);
    }
  }

  void _showPairError(ApiException e, PairTarget server) {
    final details = e.details ?? const {};
    setState(() {
      switch (e.code) {
        case 'pairing_code_invalid':
          final left = details['attempts_left'];
          _codeError = left is int
              ? _l10n.errorCodeInvalid(left)
              : _l10n.errorCodeInvalidNoCount;
          _code.clear();
        case 'pairing_code_expired':
          _codeError = _l10n.errorCodeExpired;
          _code.clear();
        case 'pairing_too_many_attempts':
          _code.clear();
          final wait = details['retry_after_s'];
          if (wait is int && wait > 0) {
            _blockedUntil = clock.now().add(Duration(seconds: wait));
            _codeError = _l10n.errorTooManyAttempts(wait);
            _ticker?.cancel();
            _ticker =
                Timer.periodic(const Duration(seconds: 1), (_) => _tick());
          } else {
            _codeError = _l10n.errorTooManyAttemptsNoDelay;
          }
        default:
          _codeError = e.isNetworkError
              ? _l10n.errorUnreachable(server.label)
              : _l10n.errorUnexpected(e.message);
      }
    });
  }

  void _tick() {
    if (!mounted) return;
    final left = _blockedLeft;
    setState(() {
      if (left <= 0) {
        _ticker?.cancel();
        _blockedUntil = null;
        _codeError = null;
        _code.clear(); // la TV affiche un nouveau code : on repart de zéro
      } else {
        _codeError = _l10n.errorTooManyAttempts(left);
      }
    });
  }

  // ------------------------------------------------------------------- vues

  @override
  Widget build(BuildContext context) {
    return PopScope(
      canPop: _step != _Step.editServer,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _closeServer();
      },
      child: switch (_step) {
        _Step.connecting => _connectingView(),
        _Step.offline => _offlineView(),
        _Step.pair => _pairView(),
        _Step.editServer => _serverView(),
      },
    );
  }

  /// Gloubi plus petit quand le clavier prend la moitié de l'écran.
  double get _gloubiSize =>
      MediaQuery.viewInsetsOf(context).bottom > 0 ? 100 : 170;

  Widget _gloubi({bool worried = false}) {
    final size = _gloubiSize;
    if (worried) {
      return Gloubi(
          key: const ValueKey('worried'),
          animation: GloubiAnimation.worried,
          size: size);
    }
    if (_greet) {
      // Arrivée dans l'app : Gloubi sort du sol et dit bonjour.
      return Gloubi.greeting(key: const ValueKey('greeting'), size: size);
    }
    return Gloubi(
        key: const ValueKey('idle'), animation: GloubiAnimation.idle, size: size);
  }

  Widget? get _serverLine => _server == null
      ? null
      : ServerLine(
          address: _server!.label,
          onChange: _sending ? null : _openServer,
        );

  Widget _connectingView() {
    return RemotePage(
      footer: _serverLine,
      children: [
        _gloubi(),
        const SizedBox(height: AppSpacing.md),
        Text(_l10n.connectingServer, style: RemoteText.title),
        const SizedBox(height: AppSpacing.md),
        CircularProgressIndicator(semanticsLabel: _l10n.connectingServer),
      ],
    );
  }

  Widget _offlineView() {
    return RemotePage(
      footer: _serverLine,
      actions: [
        BrandButton(
          label: _l10n.retry,
          icon: Icons.refresh_rounded,
          onPressed: _connect,
        ),
        GhostButton(
          label: _l10n.changeAddress,
          icon: Icons.dns_rounded,
          onPressed: _openServer,
        ),
      ],
      children: [
        // Serveur injoignable : Gloubi s'inquiète (en boucle).
        _gloubi(worried: true),
        const SizedBox(height: AppSpacing.md),
        PageTitle(
          title: _l10n.serverOfflineTitle,
          body: _l10n.serverOfflineBody(_server?.label ?? ''),
        ),
      ],
    );
  }

  Widget _pairView() {
    final blocked = _blockedLeft > 0;
    final error =
        _codeError ?? (_sessionExpired ? _l10n.sessionExpired : null);
    return RemotePage(
      footer: _serverLine,
      actions: [
        ListenableBuilder(
          listenable: _code,
          builder: (context, _) => BrandButton(
            label: _sending ? _l10n.pairing : _l10n.connect,
            icon: Icons.link_rounded,
            loading: _sending,
            onPressed: _code.text.length == 6 && !blocked ? _pair : null,
          ),
        ),
      ],
      children: [
        _gloubi(worried: error != null),
        const SizedBox(height: AppSpacing.md),
        PageTitle(title: _l10n.welcomeTitle, body: _l10n.welcomeBody),
        // Au-dessus des tuiles : reste visible clavier ouvert.
        if (error != null) ...[
          const SizedBox(height: AppSpacing.md),
          ErrorMessage(error),
        ] else if (_hint != null) ...[
          const SizedBox(height: AppSpacing.md),
          Text(_hint!, style: RemoteText.body, textAlign: TextAlign.center),
        ],
        const SizedBox(height: AppSpacing.md),
        GhostButton(
          label: _l10n.scanQr,
          icon: Icons.qr_code_scanner_rounded,
          onPressed: _sending ? null : _scan,
        ),
        const SizedBox(height: AppSpacing.sm),
        Text(_l10n.orTypeCode, style: RemoteText.muted),
        const SizedBox(height: AppSpacing.sm),
        CodeInput(
          controller: _code,
          focusNode: _codeFocus,
          semanticLabel: _l10n.codeSemantics,
          enabled: !_sending && !blocked,
          hasError: _codeError != null,
          onCompleted: (_) => _pair(),
        ),
        const SizedBox(height: AppSpacing.md),
      ],
    );
  }

  Widget _serverView() {
    final def = _defaultServer;
    final current = _server;
    final showReset = def != null &&
        (current == null ||
            current.host != def.host ||
            current.port != def.port);
    return RemotePage(
      onBack: _server == null ? null : _closeServer,
      actions: [
        BrandButton(
          label: _l10n.save,
          icon: Icons.check_rounded,
          onPressed: _saveAddress,
        ),
        if (showReset)
          GhostButton(
            label: _l10n.resetServer(def.label),
            icon: Icons.restore_rounded,
            onPressed: () => _useServer(def),
          ),
      ],
      children: [
        _gloubi(worried: _addressError != null),
        const SizedBox(height: AppSpacing.md),
        PageTitle(title: _l10n.serverTitle, body: _l10n.serverBody),
        const SizedBox(height: AppSpacing.md),
        TextField(
          key: const ValueKey('address-field'),
          controller: _address,
          keyboardType: TextInputType.url,
          textInputAction: TextInputAction.done,
          autocorrect: false,
          enableSuggestions: false,
          decoration: InputDecoration(
            hintText: _l10n.addressHint,
            labelText: _l10n.serverTitle,
            errorText: _addressError,
            prefixIcon: const Icon(Icons.dns_rounded),
          ),
          onChanged: (_) {
            if (_addressError != null) setState(() => _addressError = null);
          },
          onSubmitted: (_) => _saveAddress(),
        ),
        const SizedBox(height: AppSpacing.md),
      ],
    );
  }
}

/// « Serveur : 192.168.1.42 · Changer », en petit en bas de l'écran.
class ServerLine extends StatelessWidget {
  const ServerLine({super.key, required this.address, required this.onChange});

  final String address;
  final VoidCallback? onChange;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        const Icon(Icons.dns_rounded, size: 16, color: AppColors.textMuted),
        const SizedBox(width: 6),
        Flexible(
          child: Text(
            l10n.serverLabel(address),
            style: RemoteText.muted.copyWith(fontSize: 13),
            overflow: TextOverflow.ellipsis,
          ),
        ),
        if (onChange != null)
          Semantics(
            button: true,
            label: l10n.changeServerSemantics(address),
            excludeSemantics: true,
            child: TextButton(
              key: const ValueKey('change-server'),
              onPressed: onChange,
              style: TextButton.styleFrom(
                foregroundColor: AppColors.primary,
                textStyle:
                    const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                minimumSize: const Size(48, 40),
                padding: const EdgeInsets.symmetric(horizontal: 10),
              ),
              child: Text(l10n.changeServer),
            ),
          ),
      ],
    );
  }
}
