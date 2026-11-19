import 'dart:async';
import 'dart:math' as math;

import 'package:clock/clock.dart';
import 'package:design_system/design_system.dart';
import 'package:epikodi_client/epikodi_client.dart';
import 'package:flutter/material.dart';

import '../config.dart';
import '../deps.dart';
import '../l10n/app_localizations.dart';
import '../pairing/pairing_screen.dart';
import '../remote/dpad.dart';
import '../remote/info_screen.dart';
import '../remote/language_button.dart';
import '../remote/profile_menu.dart';
import '../remote/remote_events.dart';
import '../remote/switch_profile_sheet.dart';
import '../remote/text_input_sheet.dart';
import '../remote/tv_commands.dart';
import '../storage/remote_store.dart';
import '../theme.dart';
import '../widgets/buttons.dart';
import '../widgets/remote_page.dart';

enum _Status { loading, ok, unreachable }

/// La télécommande, une fois le téléphone appairé.
///
/// Vérifie d'abord le token (`GET /api/session`), puis affiche :
///  - en haut, le bouton du profil actif et son menu déroulant, comme sur la
///    TV (Profil, Changer de profil, Appairage), plus Infos : des raccourcis à la place
///    des flèches ;
///  - au centre, la croix directionnelle et OK ;
///  - en bas, Retour et Accueil.
///
/// Token refusé (`unauthorized`) : on oublie la TV et on revient à
/// l'appairage. TV injoignable : Gloubi s'inquiète, Réessayer ou changer.
class ConnectedScreen extends StatefulWidget {
  const ConnectedScreen({
    super.key,
    required this.pairing,
    this.client,
    this.justPaired = false,
  });

  final SavedPairing pairing;

  /// Client déjà configuré (juste après l'appairage). Sinon, l'écran en crée
  /// un avec le token gardé.
  final EpikodiClient? client;

  /// Vrai juste après l'appairage : la télécommande s'affiche tout de suite.
  final bool justPaired;

  @override
  State<ConnectedScreen> createState() => _ConnectedScreenState();
}

class _ConnectedScreenState extends State<ConnectedScreen>
    with WidgetsBindingObserver {
  EpikodiClient? _client;
  late TvCommands _commands;
  late _Status _status = widget.justPaired ? _Status.ok : _Status.loading;
  SessionProfile? _profile;
  bool _refreshing = false;
  bool _leaving = false;

  /// Événements du serveur (champ texte de la TV -> clavier du téléphone).
  RemoteEventsSource? _events;

  /// Champ texte qui a le focus sur la TV (`null` : aucun).
  final _textRequest = ValueNotifier<TextInputRequest?>(null);
  bool _keyboardOpen = false;

  /// Au lancement, Gloubi sort du sol avant de dire bonjour.
  final DateTime _arrivalUntil =
      clock.now().add(GloubiAnimation.arrival.duration);

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_client != null) return;
    final deps = RemoteDeps.of(context);
    final p = widget.pairing;
    final client = widget.client ??
        (deps.createClient(PairTarget(host: p.host, port: p.port).baseUrl)
          ..token = p.token);
    _client = client;
    _commands = deps.commands(client);
    _events = deps.events(client, _onServerEvent)..start();
    _load();
  }

  /// Un champ texte prend le focus sur la TV : le clavier du téléphone
  /// s'ouvre. Il se ferme quand la TV quitte le champ.
  void _onServerEvent(Map<String, dynamic> event) {
    if (event['type'] != 'text_input' || !mounted) return;
    final request = TextInputRequest.fromEvent(event);
    _textRequest.value = request;
    if (request != null && !_keyboardOpen && _status == _Status.ok) {
      _openKeyboard();
    }
  }

  Future<void> _openKeyboard() async {
    _keyboardOpen = true;
    await showTextInputSheet(
      context,
      request: _textRequest,
      onChanged: (value) => _commands.text(value),
      onDone: (value) => _commands.text(value, done: true),
    );
    _keyboardOpen = false;
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _events?.close();
    _textRequest.dispose();
    _client?.close();
    super.dispose();
  }

  /// Retour dans l'app : le profil a pu changer sur la TV.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed && _status != _Status.loading) {
      _load();
    }
  }

  String get _address =>
      PairTarget(host: widget.pairing.host, port: widget.pairing.port).label;

  Future<void> _load() async {
    if (_refreshing) return;
    setState(() => _refreshing = true);
    try {
      final session = await _client!.remote.session();
      await _waitForArrival();
      if (!mounted) return;
      setState(() {
        _profile = session.profile;
        _status = _Status.ok;
      });
    } on ApiException catch (e) {
      if (!mounted) return;
      if (e.code == 'unauthorized') {
        await _forget(sessionExpired: true);
        return;
      }
      await _waitForArrival();
      if (mounted) setState(() => _status = _Status.unreachable);
    } finally {
      if (mounted) setState(() => _refreshing = false);
    }
  }

  /// Laisse Gloubi finir son arrivée avant le « coucou » (lancement de l'app).
  Future<void> _waitForArrival() async {
    if (widget.justPaired || _status != _Status.loading) return;
    if (MediaQuery.disableAnimationsOf(context)) return;
    final left = _arrivalUntil.difference(clock.now());
    if (left > Duration.zero) await Future<void>.delayed(left);
  }

  /// Oublie la TV et revient à la connexion.
  Future<void> _forget({bool sessionExpired = false}) async {
    if (_leaving) return;
    _leaving = true;
    await RemoteDeps.of(context).store.clear();
    if (!mounted) return;
    Navigator.of(context).pushReplacement(MaterialPageRoute(
      builder: (_) => PairingScreen(sessionExpired: sessionExpired),
    ));
  }

  Future<void> _confirmPairing() async {
    final l10n = AppLocalizations.of(context)!;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.pairingConfirmTitle, style: RemoteText.title),
        content: Text(l10n.pairingConfirmBody, style: RemoteText.muted),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(l10n.cancel),
          ),
          TextButton(
            key: const ValueKey('confirm-pairing'),
            onPressed: () => Navigator.of(ctx).pop(true),
            style: TextButton.styleFrom(foregroundColor: AppColors.error),
            child: Text(l10n.confirm),
          ),
        ],
      ),
    );
    if (ok != true) return;
    // La TV revient aussi à son écran d'appairage. Si elle ne répond pas,
    // on se désappaire quand même côté téléphone.
    await _commands.unpair().timeout(const Duration(seconds: 3),
        onTimeout: () {});
    await _forget();
  }

  Future<void> _switchProfile() async {
    final profile = await showSwitchProfileSheet(
      context,
      client: _client!,
      current: _profile,
    );
    if (profile != null && mounted) setState(() => _profile = profile);
  }

  void _onMenu(ProfileMenuAction action) {
    switch (action) {
      case ProfileMenuAction.profile:
        _commands.shortcut(TvShortcut.profile);
      case ProfileMenuAction.switchProfile:
        _switchProfile();
      case ProfileMenuAction.pairing:
        _confirmPairing();
      case ProfileMenuAction.info:
        Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => InfoScreen(
            pairing: widget.pairing,
            profile: _profile,
            commands: _commands,
          ),
        ));
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return switch (_status) {
      _Status.loading => RemotePage(children: [
          // Lancement de l'app : Gloubi arrive pendant la vérification.
          const Gloubi.entrance(key: ValueKey('entrance'), size: 170),
          const SizedBox(height: AppSpacing.md),
          Text(l10n.checking, style: RemoteText.title),
          const SizedBox(height: AppSpacing.md),
          CircularProgressIndicator(semanticsLabel: l10n.checking),
        ]),
      _Status.unreachable => RemotePage(
          actions: [
            BrandButton(
              label: l10n.retry,
              icon: Icons.refresh_rounded,
              loading: _refreshing,
              onPressed: _load,
            ),
            GhostButton(
              label: l10n.changeTv,
              icon: Icons.swap_horiz_rounded,
              onPressed: _confirmPairing,
            ),
          ],
          children: [
            // Pas de connexion : Gloubi s'inquiète (en boucle).
            const Gloubi(
                key: ValueKey('worried'),
                animation: GloubiAnimation.worried,
                size: 170),
            const SizedBox(height: AppSpacing.md),
            PageTitle(
              title: l10n.tvUnreachableTitle,
              body: l10n.tvUnreachableBody(_address),
            ),
          ],
        ),
      _Status.ok => _remote(l10n),
    };
  }

  String? _profileName(AppLocalizations l10n) {
    final p = _profile;
    if (p == null) return null;
    return p.isGuest ? l10n.guestProfile : p.name;
  }

  Widget _remote(AppLocalizations l10n) {
    return Scaffold(
      backgroundColor: AppColors.background,
      body: BrandBackground(
        child: SafeArea(
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: maxContentWidth),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: AppSpacing.md),
                child: Column(
                  children: [
                    const SizedBox(height: AppSpacing.md),
                    // Logo au centre : Gloubi fait coucou une fois connecté.
                    // scaleDown : rétrécit sur les petits écrans.
                    const FittedBox(
                      fit: BoxFit.scaleDown,
                      child: GloubiLogo(
                        size: 44,
                        mascot: Gloubi.hello(key: ValueKey('hello'), size: 44),
                      ),
                    ),
                    const SizedBox(height: AppSpacing.md),
                    // Profil (avatar + flèche du menu) et langue, au centre.
                    SizedBox(
                      height: 56,
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          ProfileMenuButton(
                            name: _profileName(l10n),
                            onSelected: _onMenu,
                          ),
                          const SizedBox(width: AppSpacing.sm),
                          LanguageToggle(onLanguage: _commands.language),
                        ],
                      ),
                    ),
                    Expanded(
                      child: LayoutBuilder(builder: (context, c) {
                        // Pavé aussi grand que possible, barre du bas comprise.
                        final pad = math.min(
                            math.min(c.maxWidth, 340.0),
                            math.max(200.0, c.maxHeight - 58 - 64));
                        return SingleChildScrollView(
                          child: ConstrainedBox(
                            constraints:
                                BoxConstraints(minHeight: c.maxHeight),
                            child: Column(
                              mainAxisAlignment: MainAxisAlignment.center,
                              children: [
                                DPad(size: pad, onKey: _commands.key),
                                const SizedBox(height: AppSpacing.lg),
                                SizedBox(
                                  width: pad,
                                  child: NavKeys(onKey: _commands.key),
                                ),
                              ],
                            ),
                          ),
                        );
                      }),
                    ),
                    const SizedBox(height: AppSpacing.sm),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
