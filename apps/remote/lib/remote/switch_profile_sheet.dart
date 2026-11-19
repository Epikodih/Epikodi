import 'dart:async';

import 'package:clock/clock.dart';
import 'package:design_system/design_system.dart';
import 'package:epikodi_client/epikodi_client.dart';
import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme.dart';
import '../widgets/code_input.dart';
import '../widgets/profile_avatar.dart';
import '../widgets/remote_page.dart';

/// « Changer de profil » depuis le téléphone : liste des comptes
/// (`GET /api/profiles`), PIN si le compte en demande un, puis
/// `POST /api/session/profile`. Renvoie le nouveau profil actif, ou `null`
/// si l'utilisateur ferme la feuille.
Future<SessionProfile?> showSwitchProfileSheet(
  BuildContext context, {
  required EpikodiClient client,
  required SessionProfile? current,
}) =>
    showModalBottomSheet<SessionProfile>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      backgroundColor: AppColors.surface,
      shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(AppRadius.lg))),
      builder: (_) => _SwitchProfileSheet(client: client, current: current),
    );

class _SwitchProfileSheet extends StatefulWidget {
  const _SwitchProfileSheet({required this.client, required this.current});

  final EpikodiClient client;
  final SessionProfile? current;

  @override
  State<_SwitchProfileSheet> createState() => _SwitchProfileSheetState();
}

class _SwitchProfileSheetState extends State<_SwitchProfileSheet> {
  late Future<ProfileList> _list = widget.client.remote.profiles();

  /// Compte qui attend son PIN (null : on montre la liste).
  ProfileSummary? _pinFor;
  final _pin = TextEditingController();
  final _pinFocus = FocusNode();
  String? _error;
  bool _sending = false;

  DateTime? _lockedUntil;
  Timer? _ticker;

  @override
  void dispose() {
    _ticker?.cancel();
    _pin.dispose();
    _pinFocus.dispose();
    super.dispose();
  }

  AppLocalizations get _l10n => AppLocalizations.of(context)!;

  int get _lockedLeft {
    final until = _lockedUntil;
    if (until == null) return 0;
    final ms = until.difference(clock.now()).inMilliseconds;
    return ms <= 0 ? 0 : (ms / 1000).ceil();
  }

  void _choose(ProfileSummary p) {
    if (p.requiresPin) {
      setState(() {
        _pinFor = p;
        _error = null;
        _pin.clear();
      });
      WidgetsBinding.instance
          .addPostFrameCallback((_) => _pinFocus.requestFocus());
    } else {
      _select(profileId: p.id);
    }
  }

  Future<void> _select({int? profileId, String? pin, bool guest = false}) async {
    if (_sending || _lockedLeft > 0) return;
    setState(() {
      _sending = true;
      _error = null;
    });
    try {
      final profile = await widget.client.remote
          .selectProfile(profileId: profileId, pin: pin, guest: guest);
      if (mounted) Navigator.of(context).pop(profile);
    } on ApiException catch (e) {
      if (mounted) _showError(e);
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  void _showError(ApiException e) {
    final details = e.details ?? const {};
    setState(() {
      _pin.clear();
      switch (e.code) {
        case 'pin_invalid':
          final left = details['attempts_left'];
          _error =
              left is int ? _l10n.pinInvalid(left) : _l10n.pinInvalidNoCount;
        case 'pin_locked':
          final wait = details['retry_after_s'];
          if (wait is int && wait > 0) {
            _lockedUntil = clock.now().add(Duration(seconds: wait));
            _error = _l10n.pinLocked(wait);
            _ticker?.cancel();
            _ticker = Timer.periodic(const Duration(seconds: 1), (_) {
              if (!mounted) return;
              final left = _lockedLeft;
              setState(() {
                if (left <= 0) {
                  _ticker?.cancel();
                  _lockedUntil = null;
                  _error = null;
                } else {
                  _error = _l10n.pinLocked(left);
                }
              });
            });
          } else {
            _error = _l10n.pinLockedNoDelay;
          }
        default:
          _error = e.message;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      // Laisse la place au clavier (saisie du PIN).
      padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(context).bottom),
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(
            AppSpacing.md, AppSpacing.sm, AppSpacing.md, AppSpacing.md),
        child: _pinFor == null ? _listView() : _pinView(_pinFor!),
      ),
    );
  }

  Widget _handle() => Center(
        child: Container(
          width: 40,
          height: 4,
          margin: const EdgeInsets.only(bottom: AppSpacing.sm),
          decoration: BoxDecoration(
            color: AppColors.surfaceHigh,
            borderRadius: BorderRadius.circular(2),
          ),
        ),
      );

  Widget _listView() {
    return FutureBuilder<ProfileList>(
      future: _list,
      builder: (context, snap) {
        final children = <Widget>[
          _handle(),
          Semantics(
            header: true,
            child: Text(_l10n.whoIsWatching,
                style: RemoteText.headline, textAlign: TextAlign.center),
          ),
          const SizedBox(height: AppSpacing.md),
        ];
        if (snap.connectionState != ConnectionState.done) {
          children.add(const Center(
            child: Padding(
              padding: EdgeInsets.all(AppSpacing.lg),
              child: CircularProgressIndicator(),
            ),
          ));
        } else if (snap.hasError) {
          children.addAll([
            const Center(
                child: Gloubi(animation: GloubiAnimation.worried, size: 110)),
            const SizedBox(height: AppSpacing.sm),
            ErrorMessage(_l10n.profilesError),
            const SizedBox(height: AppSpacing.sm),
            TextButton(
              onPressed: () => setState(() {
                _list = widget.client.remote.profiles();
              }),
              child: Text(_l10n.retry),
            ),
          ]);
        } else {
          final list = snap.data!;
          final current = widget.current;
          if (_error != null) {
            children.addAll([ErrorMessage(_error!), const SizedBox(height: 8)]);
          }
          for (final p in list.profiles) {
            children.add(_ProfileTile(
              key: ValueKey('profile-${p.id}'),
              name: p.name,
              locked: p.requiresPin,
              current: current != null && !current.isGuest && current.id == p.id,
              enabled: !_sending,
              onTap: () => _choose(p),
            ));
          }
          if (list.guestAvailable) {
            children.add(_ProfileTile(
              key: const ValueKey('profile-guest'),
              name: _l10n.guestProfile,
              guest: true,
              current: current?.isGuest ?? false,
              enabled: !_sending,
              onTap: () => _select(guest: true),
            ));
          }
          if (_sending) {
            children.add(const Padding(
              padding: EdgeInsets.only(top: AppSpacing.sm),
              child: Center(child: CircularProgressIndicator()),
            ));
          }
        }
        return Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: children,
        );
      },
    );
  }

  Widget _pinView(ProfileSummary p) {
    final locked = _lockedLeft > 0;
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _handle(),
        Row(
          children: [
            IconButton(
              tooltip: MaterialLocalizations.of(context).backButtonTooltip,
              onPressed: _sending
                  ? null
                  : () => setState(() {
                        _pinFor = null;
                        _error = null;
                      }),
              icon: const Icon(Icons.arrow_back_rounded, color: AppColors.text),
            ),
            const SizedBox(width: AppSpacing.xs),
            ProfileAvatar(name: p.name, size: 40),
            const SizedBox(width: AppSpacing.sm),
            Expanded(
              child: Text(_l10n.pinTitle(p.name),
                  style: RemoteText.title, overflow: TextOverflow.ellipsis),
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.md),
        if (_error != null) ...[
          ErrorMessage(_error!),
          const SizedBox(height: AppSpacing.sm),
        ],
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: AppSpacing.lg),
          child: CodeInput(
            controller: _pin,
            focusNode: _pinFocus,
            length: 4,
            semanticLabel: _l10n.pinSemantics,
            enabled: !_sending && !locked,
            hasError: _error != null,
            onCompleted: (pin) => _select(profileId: p.id, pin: pin),
          ),
        ),
        if (_sending)
          const Padding(
            padding: EdgeInsets.only(top: AppSpacing.md),
            child: Center(child: CircularProgressIndicator()),
          ),
        const SizedBox(height: AppSpacing.sm),
      ],
    );
  }
}

class _ProfileTile extends StatelessWidget {
  const _ProfileTile({
    super.key,
    required this.name,
    required this.onTap,
    required this.enabled,
    this.locked = false,
    this.current = false,
    this.guest = false,
  });

  final String name;
  final VoidCallback onTap;
  final bool enabled;
  final bool locked;
  final bool current;
  final bool guest;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.only(bottom: AppSpacing.xs),
      child: Material(
        color: current ? AppColors.surfaceHigh : AppColors.background,
        borderRadius: BorderRadius.circular(AppRadius.md),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: enabled ? onTap : null,
          child: Padding(
            padding: const EdgeInsets.symmetric(
                horizontal: AppSpacing.sm, vertical: AppSpacing.sm - 4),
            child: Row(
              children: [
                ProfileAvatar(name: guest ? null : name, size: 44),
                const SizedBox(width: AppSpacing.sm),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(name,
                          style: RemoteText.body
                              .copyWith(fontWeight: FontWeight.w600)),
                      if (current)
                        Text(l10n.currentProfile,
                            style: RemoteText.muted.copyWith(fontSize: 13)),
                    ],
                  ),
                ),
                if (locked)
                  Tooltip(
                    message: l10n.profileLocked,
                    child: const Icon(Icons.lock_rounded,
                        size: 20, color: AppColors.textMuted),
                  ),
                if (current)
                  const Padding(
                    padding: EdgeInsets.only(left: AppSpacing.xs),
                    child: Icon(Icons.check_circle_rounded,
                        color: AppColors.primary),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
