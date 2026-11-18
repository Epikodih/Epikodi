import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../widgets/language_button.dart';
import '../widgets/pairing_button.dart';
import '../widgets/pill_metrics.dart';

/// Écran « Qui regarde ? » : affiche les comptes envoyés par le serveur
/// (champ `profiles`), puis « Créer » si le serveur l'autorise (`can_create`)
/// et « Invité » si le serveur l'autorise (`guest.available`).
///
/// - Les cartes sont toujours sur une seule ligne, qui défile horizontalement
///   quand il y a beaucoup de comptes (la télécommande suit le focus).
/// - La taille des cartes s'adapte à la hauteur disponible : plus d'overflow
///   quand la fenêtre est petite.
/// - Sans aucun compte, un texte explique : créer un compte ou regarder en invité.
///
/// L'écran ne fait aucun appel réseau : il prévient le parent via
/// [onProfile], [onCreate] et [onGuest] (voir `accounts_flow.dart`).
/// En haut à droite : bouton pour revenir à l'appairage, puis sélecteur de langue.
class AccountsScreen extends StatelessWidget {
  const AccountsScreen({
    super.key,
    required this.data,
    required this.onBackToPairing,
    required this.onProfile,
    required this.onCreate,
    required this.onGuest,
  });

  /// Réponse du serveur : { phase, profiles: [...], guest: {...}, can_create }.
  final Map<String, dynamic> data;

  /// Retour à l'écran d'appairage (bouton en haut à droite).
  final VoidCallback onBackToPairing;

  /// Un compte existant a été choisi.
  final void Function(Map<String, dynamic> profile) onProfile;

  /// La carte « Créer » a été choisie.
  final VoidCallback onCreate;

  /// La carte « Invité » a été choisie.
  final VoidCallback onGuest;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final profiles = (data['profiles'] as List? ?? const [])
        .whereType<Map<String, dynamic>>()
        .toList();
    final guestAvailable = (data['guest'] as Map?)?['available'] == true;
    final canCreate = data['can_create'] == true;
    final noAccount = profiles.isEmpty;

    // Hauteur commune aux boutons de l'en-tête (Appairage + langue).
    final buttonHeight =
        MediaQuery.textScalerOf(context).scale(pillButtonHeight);

    // Construit les cartes pour une taille d'avatar donnée (calculée selon la fenêtre).
    List<Widget> buildCards(double avatar) {
      // Le premier élément de la ligne prend le focus au départ.
      var focusGiven = false;
      bool takeFocus() {
        if (focusGiven) return false;
        focusGiven = true;
        return true;
      }

      return [
        for (final p in profiles)
          _AccountCard(
            name: '${p['name']}',
            avatarSize: avatar,
            locked: p['requires_pin'] == true,
            autofocus: takeFocus(),
            onTap: () => onProfile(p),
          ),
        if (canCreate)
          _AccountCard(
            name: l10n.createCard,
            avatarSize: avatar,
            icon: Icons.add_rounded,
            semanticLabel: l10n.createAccount,
            autofocus: takeFocus(),
            onTap: onCreate,
          ),
        if (guestAvailable)
          _AccountCard(
            name: l10n.guestCard,
            avatarSize: avatar,
            icon: Icons.person_outline_rounded,
            semanticLabel: l10n.continueAsGuest,
            autofocus: takeFocus(),
            onTap: onGuest,
          ),
      ];
    }

    return Scaffold(
      body: BrandBackground(
        child: TvSafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.lg,
              vertical: AppSpacing.md,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const GloubiLogo(size: 56),
                    const SizedBox(width: AppSpacing.md),
                    // Wrap : si la fenêtre est étroite, les boutons passent
                    // à la ligne au lieu de déborder.
                    Expanded(
                      child: Wrap(
                        alignment: WrapAlignment.end,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        spacing: AppSpacing.md,
                        runSpacing: AppSpacing.sm,
                        children: [
                          PairingButton(onTap: onBackToPairing),
                          LanguageButton(height: buttonHeight),
                        ],
                      ),
                    ),
                  ],
                ),
                Expanded(
                  // Titre + ligne de cartes centrés verticalement ; la ligne
                  // prend la hauteur qui reste (Flexible) et adapte ses cartes.
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Text(
                        noAccount ? l10n.noAccountYet : l10n.whoIsWatching,
                        style: AppText.headline,
                        textAlign: TextAlign.center,
                      ),
                      if (noAccount) ...[
                        const SizedBox(height: AppSpacing.sm),
                        Text(
                          l10n.createOrGuest,
                          style:
                              AppText.body.copyWith(color: AppColors.textMuted),
                          textAlign: TextAlign.center,
                        ),
                      ],
                      const SizedBox(height: AppSpacing.md),
                      Flexible(child: _AccountsRow(builder: buildCards)),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Une seule ligne de cartes, centrée si elle tient à l'écran,
/// qui défile horizontalement sinon. Calcule la taille d'avatar pour que
/// les cartes tiennent dans la hauteur disponible.
class _AccountsRow extends StatelessWidget {
  const _AccountsRow({required this.builder});

  final List<Widget> Function(double avatarSize) builder;

  @override
  Widget build(BuildContext context) {
    final scaler = MediaQuery.textScalerOf(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        const sidePadding = AppSpacing.lg;
        const verticalPadding = AppSpacing.md;

        // Hauteur d'une carte = avatar + marges de la carte + espace + libellé.
        final labelHeight = scaler.scale(
            (AppText.title.fontSize ?? 24) * (AppText.title.height ?? 1.3));
        final chrome = 2 * verticalPadding + // marge du défilement
            2 * AppSpacing.md + // marge intérieure de la carte
            AppSpacing.sm +
            labelHeight;
        final maxAvatar = scaler.scale(112);
        final minAvatar = scaler.scale(56);
        final avatar = constraints.maxHeight.isFinite
            ? (constraints.maxHeight - chrome)
                .clamp(minAvatar, maxAvatar)
                .toDouble()
            : maxAvatar;

        final children = builder(avatar);
        return SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          // Marge autour des cartes : le halo de focus n'est pas coupé.
          padding: const EdgeInsets.symmetric(
            horizontal: sidePadding,
            vertical: verticalPadding,
          ),
          child: ConstrainedBox(
            constraints: BoxConstraints(
              minWidth: constraints.maxWidth - 2 * sidePadding,
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                for (var i = 0; i < children.length; i++) ...[
                  if (i > 0) const SizedBox(width: AppSpacing.lg),
                  children[i],
                ],
              ],
            ),
          ),
        );
      },
    );
  }
}

/// Carte d'un compte. Avec `icon`, c'est une carte d'action (Créer, Invité) :
/// pastille grise avec l'icône au lieu de l'initiale.
/// Le contenu est dans un FittedBox : si la place manque, il se réduit
/// au lieu de provoquer un « bottom overflow ».
class _AccountCard extends StatelessWidget {
  const _AccountCard({
    required this.name,
    required this.onTap,
    required this.avatarSize,
    this.locked = false,
    this.icon,
    this.semanticLabel,
    this.autofocus = false,
  });

  final String name;
  final VoidCallback onTap;
  final double avatarSize;
  final bool locked;
  final IconData? icon;
  final String? semanticLabel;
  final bool autofocus;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final scaler = MediaQuery.textScalerOf(context);
    final isAction = icon != null;
    // Même proportion qu'avant (220 pour un avatar de 112).
    final cardWidth = avatarSize * 1.96;
    final lockWidth = locked ? scaler.scale(28) + AppSpacing.xs : 0.0;
    final labelMaxWidth = cardWidth - 2 * AppSpacing.md - lockWidth;

    return SizedBox(
      width: cardWidth,
      child: FocusableCard(
        autofocus: autofocus,
        onTap: onTap,
        borderRadius: AppRadius.lg,
        padding: const EdgeInsets.all(AppSpacing.md),
        semanticLabel: semanticLabel ??
            (locked
                ? l10n.accountLockedSemantics(name)
                : l10n.accountSemantics(name)),
        child: FittedBox(
          fit: BoxFit.scaleDown,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: avatarSize,
                height: avatarSize,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  gradient: isAction ? null : AppBrand.gradient,
                  color: isAction ? AppColors.surfaceHigh : null,
                  shape: BoxShape.circle,
                ),
                child: isAction
                    ? Icon(icon, size: avatarSize * 0.55, color: AppColors.text)
                    : FittedBox(
                        child: Padding(
                          padding: EdgeInsets.all(avatarSize * 0.2),
                          child: Text(
                            name.isEmpty
                                ? '?'
                                : name.characters.first.toUpperCase(),
                            style:
                                AppText.headline.copyWith(color: Colors.white),
                          ),
                        ),
                      ),
              ),
              const SizedBox(height: AppSpacing.sm),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  ConstrainedBox(
                    constraints: BoxConstraints(maxWidth: labelMaxWidth),
                    child: Text(
                      name,
                      style: AppText.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  // Le code PIN est indiqué par une icône, pas par la seule couleur.
                  if (locked) ...[
                    const SizedBox(width: AppSpacing.xs),
                    Icon(Icons.lock_rounded,
                        size: scaler.scale(28), color: AppColors.textMuted),
                  ],
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
