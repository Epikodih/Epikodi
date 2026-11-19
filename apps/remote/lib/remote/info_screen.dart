import 'package:design_system/design_system.dart';
import 'package:epikodi_client/epikodi_client.dart';
import 'package:flutter/material.dart';

import '../config.dart';
import '../deps.dart';
import '../l10n/app_localizations.dart';
import '../pairing/pairing_screen.dart';
import '../remote/tv_commands.dart';
import '../storage/remote_store.dart';
import '../theme.dart';
import '../widgets/buttons.dart';
import '../widgets/remote_page.dart';

/// « Infos » (menu du profil) : adresse du serveur, téléphone, profil actif,
/// version. L'adresse se change ici : le téléphone quitte alors la TV
/// actuelle et se connecte au nouveau serveur (nouvel appairage).
class InfoScreen extends StatefulWidget {
  const InfoScreen({
    super.key,
    required this.pairing,
    this.profile,
    this.commands,
  });

  final SavedPairing pairing;
  final SessionProfile? profile;

  /// Pour désappairer la TV actuelle avant de changer de serveur.
  final TvCommands? commands;

  @override
  State<InfoScreen> createState() => _InfoScreenState();
}

class _InfoScreenState extends State<InfoScreen> {
  late final _address = TextEditingController(text: _current.label);
  String? _error;

  PairTarget get _current =>
      PairTarget(host: widget.pairing.host, port: widget.pairing.port);

  @override
  void dispose() {
    _address.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final l10n = AppLocalizations.of(context)!;
    final target = PairTarget.parse(_address.text);
    if (target == null) {
      setState(() => _error = l10n.addressInvalid);
      return;
    }
    if (target.host == _current.host && target.port == _current.port) {
      Navigator.of(context).pop();
      return;
    }
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.changeServerConfirmTitle, style: RemoteText.title),
        content: Text(l10n.changeServerConfirmBody, style: RemoteText.muted),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(l10n.cancel),
          ),
          TextButton(
            key: const ValueKey('confirm-server'),
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(l10n.confirm),
          ),
        ],
      ),
    );
    if (ok != true || !mounted) return;

    final deps = RemoteDeps.of(context);
    await widget.commands?.unpair().timeout(const Duration(seconds: 3),
        onTimeout: () {});
    if (!mounted) return;
    final def = PairTarget.parse(deps.defaultServer);
    final isDefault =
        def != null && def.host == target.host && def.port == target.port;
    await deps.store
        .setServerAddress(isDefault ? null : '${target.host}:${target.port}');
    await deps.store.clear();
    if (!mounted) return;
    Navigator.of(context).pushAndRemoveUntil(
      MaterialPageRoute(builder: (_) => const PairingScreen()),
      (_) => false,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final p = widget.profile;
    final profile = p == null
        ? l10n.noProfile
        : p.isGuest
            ? l10n.guestProfile
            : p.name;
    return RemotePage(
      onBack: () => Navigator.of(context).pop(),
      actions: [
        BrandButton(
          label: l10n.save,
          icon: Icons.check_rounded,
          onPressed: _save,
        ),
      ],
      children: [
        PageTitle(title: l10n.infoTitle),
        const SizedBox(height: AppSpacing.md),
        Container(
          padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.md, vertical: AppSpacing.xs),
          decoration: BoxDecoration(
            color: AppColors.surface,
            borderRadius: BorderRadius.circular(AppRadius.lg),
            border: Border.all(color: AppColors.surfaceHigh, width: 2),
          ),
          child: Column(
            children: [
              _Row(
                  icon: Icons.smartphone_rounded,
                  label: l10n.infoPhone,
                  value: widget.pairing.deviceName),
              const Divider(color: AppColors.surfaceHigh, height: 1),
              _Row(
                  icon: Icons.person_rounded,
                  label: l10n.infoProfile,
                  value: profile),
              const Divider(color: AppColors.surfaceHigh, height: 1),
              _Row(
                  icon: Icons.info_outline_rounded,
                  label: l10n.infoVersion,
                  value: appVersion),
            ],
          ),
        ),
        const SizedBox(height: AppSpacing.lg),
        TextField(
          key: const ValueKey('address-field'),
          controller: _address,
          keyboardType: TextInputType.url,
          textInputAction: TextInputAction.done,
          autocorrect: false,
          enableSuggestions: false,
          decoration: InputDecoration(
            labelText: l10n.infoServer,
            hintText: l10n.addressHint,
            errorText: _error,
            prefixIcon: const Icon(Icons.dns_rounded),
          ),
          onChanged: (_) {
            if (_error != null) setState(() => _error = null);
          },
          onSubmitted: (_) => _save(),
        ),
        const SizedBox(height: AppSpacing.md),
      ],
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({required this.icon, required this.label, required this.value});

  final IconData icon;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: AppSpacing.sm - 2),
      child: Row(
        children: [
          Icon(icon, color: AppColors.primary, size: 22),
          const SizedBox(width: AppSpacing.sm - 4),
          Expanded(child: Text(label, style: RemoteText.muted)),
          Flexible(
            child: Text(value,
                style: RemoteText.body.copyWith(fontWeight: FontWeight.w600),
                textAlign: TextAlign.end,
                overflow: TextOverflow.ellipsis),
          ),
        ],
      ),
    );
  }
}
