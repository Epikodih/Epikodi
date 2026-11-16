import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import 'pill_metrics.dart';
import 'profile_avatar.dart';

enum _ProfileMenuAction { profile, switchProfile, pairing }

/// Bouton du profil actif (avatar + nom + flèche). Un clic ouvre un panneau
/// aligné sur le bord droit du bouton :
///  - « Profil »             -> [onProfile]
///  - « Changer de profil »  -> [onSwitchProfile]
///  - « Appairage »          -> [onPairing]
/// Le menu est fermé avant l'appel du callback.
class ProfileMenuButton extends StatefulWidget {
  const ProfileMenuButton({
    super.key,
    required this.name,
    required this.onProfile,
    required this.onSwitchProfile,
    required this.onPairing,
    this.autofocus = false,
  });

  final String name;
  final VoidCallback onProfile;
  final VoidCallback onSwitchProfile;
  final VoidCallback onPairing;
  final bool autofocus;

  @override
  State<ProfileMenuButton> createState() => _ProfileMenuButtonState();
}

class _ProfileMenuButtonState extends State<ProfileMenuButton> {
  final _anchorKey = GlobalKey();

  Future<void> _open() async {
    final l10n = AppLocalizations.of(context)!;
    final box = _anchorKey.currentContext?.findRenderObject() as RenderBox?;
    if (box == null) return;
    final topLeft = box.localToGlobal(Offset.zero);
    // Point d'ancrage : bord DROIT du bouton, juste en dessous.
    final anchor = Offset(
      topLeft.dx + box.size.width,
      topLeft.dy + box.size.height + AppSpacing.sm,
    );
    final reduced = UiSettingsScope.of(context).reducedMotion;
    final name = widget.name;

    final action = await showGeneralDialog<_ProfileMenuAction>(
      context: context,
      barrierDismissible: true,
      barrierLabel: l10n.closeMenu,
      barrierColor: Colors.black45,
      transitionDuration:
          reduced ? Duration.zero : const Duration(milliseconds: 140),
      pageBuilder: (_, __, ___) => _MenuPanel(anchor: anchor, name: name),
      transitionBuilder: (_, animation, __, child) => FadeTransition(
        opacity: CurvedAnimation(parent: animation, curve: Curves.easeOut),
        child: child,
      ),
    );

    switch (action) {
      case _ProfileMenuAction.profile:
        widget.onProfile();
      case _ProfileMenuAction.switchProfile:
        widget.onSwitchProfile();
      case _ProfileMenuAction.pairing:
        widget.onPairing();
      case null:
        break;
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final scaler = MediaQuery.textScalerOf(context);
    final height = scaler.scale(pillButtonHeight);
    const gap = AppSpacing.xs;

    return FocusableCard(
      key: _anchorKey,
      autofocus: widget.autofocus,
      borderRadius: 999,
      onTap: _open,
      semanticLabel: l10n.profileMenuSemantics(widget.name),
      padding: const EdgeInsets.only(left: gap, right: AppSpacing.md),
      child: SizedBox(
        height: height,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            // Avatar presque à fleur de la pastille : plus de « lettre
            // dans un conteneur dans un conteneur ».
            ProfileAvatar(name: widget.name, size: height - gap * 2),
            const SizedBox(width: AppSpacing.sm),
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: scaler.scale(200)),
              child: Text(
                widget.name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: AppText.body.copyWith(fontWeight: FontWeight.w600),
              ),
            ),
            const SizedBox(width: gap),
            Icon(
              Icons.keyboard_arrow_down_rounded,
              size: scaler.scale(32),
              color: AppColors.textMuted,
            ),
          ],
        ),
      ),
    );
  }
}

/// Le panneau du menu, aligné sur le bord droit du bouton.
/// [anchor] : x = bord droit du bouton, y = juste sous le bouton.
class _MenuPanel extends StatelessWidget {
  const _MenuPanel({required this.anchor, required this.name});

  final Offset anchor;
  final String name;

  /// Largeur du texte tel qu'il sera affiché (langue + taille de texte).
  double _textWidth(BuildContext context, String text, TextStyle style) {
    final painter = TextPainter(
      text: TextSpan(text: text, style: style),
      textDirection: Directionality.of(context),
      textScaler: MediaQuery.textScalerOf(context),
      maxLines: 1,
    )..layout();
    return painter.width;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final scaler = MediaQuery.textScalerOf(context);
    final screenWidth = MediaQuery.sizeOf(context).width;
    void close() => Navigator.of(context).pop();

    // Style identique à celui des lignes (_MenuItem).
    final itemStyle = AppText.title.copyWith(
      fontWeight: FontWeight.w600,
      height: 1.3,
    );

    // Le texte le plus long parmi les 3 lignes (change avec FR / EN).
    final labels = [l10n.profile, l10n.switchProfile, l10n.pairingButton];
    final longest = labels
        .map((t) => _textWidth(context, t, itemStyle))
        .reduce((a, b) => a > b ? a : b);

    // Largeur = padding du panneau + bordure + padding de la ligne
    //           + icône + espace + texte le plus long + petite marge.
    final needed = 2 * AppSpacing.md // padding du panneau
        +
        4 // bordure (2 de chaque côté)
        +
        2 * AppSpacing.md // padding de la ligne
        +
        scaler.scale(36) // icône
        +
        AppSpacing.md // espace icône / texte
        +
        longest +
        16; // marge de sécurité (halo de focus, arrondis)

    final width = needed
        .clamp(scaler.scale(340), screenWidth - 2 * AppSpacing.md)
        .toDouble();

    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): close,
        const SingleActivator(LogicalKeyboardKey.goBack): close,
      },
      child: Material(
        type: MaterialType.transparency,
        child: FocusScope(
          child: Stack(
            children: [
              Positioned(
                right: (screenWidth - anchor.dx).clamp(0.0, screenWidth),
                top: anchor.dy,
                child: Container(
                  width: width,
                  padding: const EdgeInsets.all(AppSpacing.md),
                  decoration: BoxDecoration(
                    color: AppColors.surface,
                    borderRadius: BorderRadius.circular(AppRadius.lg),
                    border: Border.all(color: AppColors.surfaceHigh, width: 2),
                    boxShadow: const [
                      BoxShadow(
                        color: Colors.black54,
                        blurRadius: 32,
                        offset: Offset(0, 12),
                      ),
                    ],
                  ),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      // En-tête : grand avatar + nom.
                      Row(
                        children: [
                          ProfileAvatar(name: name, size: scaler.scale(72)),
                          const SizedBox(width: AppSpacing.md),
                          Expanded(
                            child: Text(
                              name,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: AppText.title,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: AppSpacing.md),
                      Divider(
                        height: 2,
                        thickness: 2,
                        color: AppColors.surfaceHigh,
                      ),
                      const SizedBox(height: AppSpacing.md),
                      _MenuItem(
                        icon: Icons.person_rounded,
                        text: l10n.profile,
                        autofocus: true,
                        action: _ProfileMenuAction.profile,
                      ),
                      const SizedBox(height: AppSpacing.sm),
                      _MenuItem(
                        icon: Icons.switch_account_rounded,
                        text: l10n.switchProfile,
                        action: _ProfileMenuAction.switchProfile,
                      ),
                      const SizedBox(height: AppSpacing.sm),
                      _MenuItem(
                        icon: Icons.qr_code_2_rounded,
                        text: l10n.pairingButton,
                        action: _ProfileMenuAction.pairing,
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _MenuItem extends StatelessWidget {
  const _MenuItem({
    required this.icon,
    required this.text,
    required this.action,
    this.autofocus = false,
  });

  final IconData icon;
  final String text;
  final _ProfileMenuAction action;
  final bool autofocus;

  @override
  Widget build(BuildContext context) {
    final scaler = MediaQuery.textScalerOf(context);
    return FocusableCard(
      autofocus: autofocus,
      borderRadius: AppRadius.md,
      onTap: () => Navigator.of(context).pop(action),
      semanticLabel: text,
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.md,
        vertical: AppSpacing.md,
      ),
      child: Row(
        children: [
          Icon(icon, size: scaler.scale(36), color: AppColors.text),
          const SizedBox(width: AppSpacing.md),
          Expanded(
            child: Text(
              text,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: AppText.title.copyWith(
                fontWeight: FontWeight.w600,
                height: 1.3,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
