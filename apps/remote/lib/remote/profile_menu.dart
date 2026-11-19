import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme.dart';
import '../widgets/profile_avatar.dart';

enum ProfileMenuAction { profile, switchProfile, pairing, info }

/// Bouton du profil actif (avatar + nom + flèche), comme sur la TV. Un appui
/// ouvre le même menu déroulant, aligné sur le bord droit du bouton :
///  - « Profil »             -> raccourci vers la page profil de la TV ;
///  - « Changer de profil »  -> choix du compte depuis le téléphone ;
///  - « Appairage »          -> déconnecte ce téléphone de la TV ;
///  - « Infos »              -> adresse du serveur (modifiable), téléphone.
class ProfileMenuButton extends StatefulWidget {
  const ProfileMenuButton({
    super.key,
    required this.name,
    required this.onSelected,
  });

  /// Nom du profil actif, `null` si aucun n'est choisi.
  final String? name;
  final ValueChanged<ProfileMenuAction> onSelected;

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
    // Point d'ancrage : juste sous le bouton. Le menu est centré sur
    // l'écran (le bouton aussi), donc jamais coupé.
    final anchor = Offset(
      topLeft.dx + box.size.width / 2,
      topLeft.dy + box.size.height + AppSpacing.xs,
    );
    final reduced = MediaQuery.disableAnimationsOf(context);
    final name = widget.name;

    final action = await showGeneralDialog<ProfileMenuAction>(
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
    if (action != null) widget.onSelected(action);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final label = widget.name ?? l10n.noProfile;
    return Semantics(
      button: true,
      label: l10n.profileMenuSemantics(label),
      excludeSemantics: true,
      child: Material(
        key: _anchorKey,
        color: AppColors.surface,
        shape: const StadiumBorder(
            side: BorderSide(color: AppColors.surfaceHigh, width: 2)),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          key: const ValueKey('profile-menu'),
          onTap: _open,
          // Juste l'avatar et la flèche : le nom est dans le menu.
          child: Padding(
            padding: const EdgeInsets.fromLTRB(6, 6, 10, 6),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                ProfileAvatar(name: widget.name, size: 42),
                const SizedBox(width: 4),
                const Icon(Icons.keyboard_arrow_down_rounded,
                    size: 28, color: AppColors.textMuted),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Le panneau du menu : sous le bouton, centré sur l'écran.
class _MenuPanel extends StatelessWidget {
  const _MenuPanel({required this.anchor, required this.name});

  final Offset anchor;
  final String? name;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final screenWidth = MediaQuery.sizeOf(context).width;
    final width = (screenWidth - 2 * AppSpacing.md).clamp(0.0, 360.0);
    final left = (screenWidth - width) / 2;

    return Material(
      type: MaterialType.transparency,
      child: Stack(
        children: [
          Positioned(
            left: left,
            top: anchor.dy,
            child: Container(
              width: width,
              padding: const EdgeInsets.all(AppSpacing.sm),
              decoration: BoxDecoration(
                color: AppColors.surface,
                borderRadius: BorderRadius.circular(AppRadius.lg),
                border: Border.all(color: AppColors.surfaceHigh, width: 2),
                boxShadow: const [
                  BoxShadow(
                      color: Colors.black54,
                      blurRadius: 32,
                      offset: Offset(0, 12)),
                ],
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // En-tête : grand avatar + nom.
                  Padding(
                    padding: const EdgeInsets.all(AppSpacing.xs),
                    child: Row(
                      children: [
                        ProfileAvatar(name: name, size: 52),
                        const SizedBox(width: AppSpacing.sm),
                        Expanded(
                          child: Text(
                            name ?? l10n.noProfile,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: RemoteText.title,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: AppSpacing.xs),
                  const Divider(
                      height: 2, thickness: 2, color: AppColors.surfaceHigh),
                  const SizedBox(height: AppSpacing.xs),
                  if (name != null)
                    _MenuItem(
                      icon: Icons.person_rounded,
                      text: l10n.profile,
                      action: ProfileMenuAction.profile,
                    ),
                  _MenuItem(
                    icon: Icons.switch_account_rounded,
                    text: l10n.switchProfile,
                    action: ProfileMenuAction.switchProfile,
                  ),
                  _MenuItem(
                    icon: Icons.qr_code_2_rounded,
                    text: l10n.pairingMenu,
                    action: ProfileMenuAction.pairing,
                  ),
                  _MenuItem(
                    icon: Icons.info_outline_rounded,
                    text: l10n.infoMenu,
                    action: ProfileMenuAction.info,
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _MenuItem extends StatelessWidget {
  const _MenuItem({
    required this.icon,
    required this.text,
    required this.action,
  });

  final IconData icon;
  final String text;
  final ProfileMenuAction action;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      key: ValueKey('menu-${action.name}'),
      borderRadius: BorderRadius.circular(AppRadius.md),
      onTap: () => Navigator.of(context).pop(action),
      child: Padding(
        padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.sm, vertical: AppSpacing.sm),
        child: Row(
          children: [
            Icon(icon, size: 26, color: AppColors.text),
            const SizedBox(width: AppSpacing.sm),
            Expanded(
              child: Text(text,
                  style: RemoteText.title.copyWith(fontSize: 17)),
            ),
          ],
        ),
      ),
    );
  }
}
