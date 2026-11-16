import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../l10n/app_localizations.dart';
import '../widgets/language_button.dart';

/// Panneau d'appairage de l'app TV (purement visuel, sans logique réseau).
/// Deux colonnes, sans défilement :
///  - à gauche : logo, titre, une phrase, puis sélecteur de langue et
///    bouton « Nouveau code » côte à côte (même hauteur) ;
///  - à droite, centrés en hauteur : QR, code en dessous, « Expire dans mm:ss ».
///
/// Les tailles de texte ne changent pas : seul le QR s'adapte à la hauteur
/// disponible (et à la largeur, pour laisser sa place à la colonne de gauche).
/// Spécifique à la TV : il reste dans apps/tv, seul le style de marque est partagé.
class PairingPanel extends StatelessWidget {
  const PairingPanel({
    super.key,
    required this.code,
    required this.qrData,
    required this.remaining,
    required this.cooldownLeft,
    required this.onNewCode,
  });

  /// Code à 6 chiffres.
  final String code;

  /// Contenu du QR (URI epikodi://pair…).
  final String qrData;

  /// Secondes restantes avant expiration du code.
  final int remaining;

  /// Secondes avant de pouvoir redemander un code (0 = disponible).
  final int cooldownLeft;

  final VoidCallback onNewCode;

  // Cadre du QR : bordure dégradée (4 × 2) + zone de silence (AppSpacing.md × 2).
  static const _qrFrame = 4.0 * 2 + AppSpacing.md * 2;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final settings = UiSettingsScope.of(context);
    final scaler = MediaQuery.textScalerOf(context);

    // Hauteur estimée de ce qui accompagne le QR dans la colonne de droite.
    final codeH = scaler.scale(96) * 1.2 + 24;
    final expireH = scaler.scale(28) * 1.2;
    final extras = _qrFrame + codeH + expireH + AppSpacing.md * 2;

    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.lg,
        vertical: AppSpacing.md,
      ),
      child: LayoutBuilder(builder: (context, c) {
        final byHeight = c.maxHeight - extras;
        // Le QR prend au plus ~45 % de la largeur : la colonne de gauche garde
        // assez de place pour le titre et le bouton, même fenêtre étroite.
        final byWidth = (c.maxWidth - AppSpacing.xl) * 0.45 - _qrFrame;
        final qrSize = (byHeight < byWidth ? byHeight : byWidth)
            .clamp(160.0, 640.0 * settings.qrScale)
            .toDouble();

        // Filet de sécurité : si malgré tout le contenu est plus haut que la
        // fenêtre (taille minimale, texte agrandi), l'ensemble est réduit d'un
        // bloc au lieu de déborder. En temps normal : taille réelle (scale 1).
        return FittedBox(
          fit: BoxFit.scaleDown,
          child: SizedBox(
            width: c.maxWidth,
            child: Row(
              crossAxisAlignment:
                  CrossAxisAlignment.center, // centré en hauteur
              children: [
                Expanded(
                  child: _Info(
                    cooldownLeft: cooldownLeft,
                    onNewCode: onNewCode,
                  ),
                ),
                const SizedBox(width: AppSpacing.xl),
                SizedBox(
                  width: qrSize + _qrFrame,
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      _QrCard(data: qrData, size: qrSize),
                      const SizedBox(height: AppSpacing.md),
                      // Le lecteur d'écran lit le code chiffre par chiffre.
                      Semantics(
                        label:
                            l10n.pairingCodeSemantics(code.split('').join(' ')),
                        excludeSemantics: true,
                        // scaleDown : ne dépasse jamais la largeur du QR.
                        child: FittedBox(
                          fit: BoxFit.scaleDown,
                          child: _CodeTiles(code: code),
                        ),
                      ),
                      const SizedBox(height: AppSpacing.md),
                      _Countdown(remaining: remaining),
                    ],
                  ),
                ),
              ],
            ),
          ),
        );
      }),
    );
  }
}

/// Colonne de gauche : logo, titre, une phrase, boutons.
class _Info extends StatelessWidget {
  const _Info({required this.cooldownLeft, required this.onNewCode});

  final int cooldownLeft;
  final VoidCallback onNewCode;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    // Hauteur commune aux deux boutons (langue + « Nouveau code »).
    final buttonHeight = MediaQuery.textScalerOf(context).scale(62);

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const EpikodiLogo(size: 56),
        const SizedBox(height: AppSpacing.lg),
        Text(l10n.pairingTitle, style: AppText.headline),
        const SizedBox(height: AppSpacing.sm),
        Text(
          l10n.pairingInstruction,
          style: AppText.body.copyWith(color: AppColors.textMuted),
        ),
        const SizedBox(height: AppSpacing.lg),
        // Le Wrap fait passer « Nouveau code » sous la langue si la colonne
        // est très étroite.
        Wrap(
          spacing: AppSpacing.md,
          runSpacing: AppSpacing.sm,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            LanguageButton(height: buttonHeight),
            _NewCodeButton(cooldownLeft: cooldownLeft, onNewCode: onNewCode),
          ],
        ),
      ],
    );
  }
}

/// Bouton « Nouveau code » en pilule dégradée.
/// Le focus (contour blanc + zoom) vient de FocusableCard.
class _NewCodeButton extends StatelessWidget {
  const _NewCodeButton({required this.cooldownLeft, required this.onNewCode});

  final int cooldownLeft;
  final VoidCallback onNewCode;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final scaler = MediaQuery.textScalerOf(context);
    final cooling = cooldownLeft > 0;

    // Pendant le délai : le bouton garde le focus mais ne réagit pas.
    // L'état est annoncé par l'icône ET le texte (pas seulement l'opacité).
    return FocusableCard(
      autofocus: true,
      borderRadius: 999,
      onTap: cooling ? null : onNewCode,
      semanticLabel:
          cooling ? l10n.newCodeCoolingSemantics : l10n.newCodeSemantics,
      child: Opacity(
        opacity: cooling ? 0.55 : 1.0,
        child: Container(
          // Hauteur fixe, identique au sélecteur de langue.
          height: scaler.scale(62),
          padding: const EdgeInsets.symmetric(horizontal: AppSpacing.lg),
          decoration: const BoxDecoration(gradient: AppBrand.gradient),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Icon(
                cooling ? Icons.hourglass_top_rounded : Icons.refresh_rounded,
                size: scaler.scale(32),
                color: Colors.white,
              ),
              const SizedBox(width: AppSpacing.xs + 4),
              // Flexible + scaleDown : si la colonne est très étroite, le libellé
              // rétrécit au lieu de déborder.
              Flexible(
                child: FittedBox(
                  fit: BoxFit.scaleDown,
                  child: Text(
                    cooling ? l10n.newCodeCooldown(cooldownLeft) : l10n.newCode,
                    maxLines: 1,
                    style: AppText.body.copyWith(
                      color: Colors.white,
                      fontWeight: FontWeight.w600,
                      height: 1.3, // évite que les jambages soient coupés
                    ),
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

/// « Expire dans mm:ss » (texte seul, pas de barre).
class _Countdown extends StatelessWidget {
  const _Countdown({required this.remaining});

  final int remaining;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final m = (remaining ~/ 60).toString().padLeft(2, '0');
    final s = (remaining % 60).toString().padLeft(2, '0');
    // Annonce vocale à la minute (pas chaque seconde) pour ne pas spammer.
    return Semantics(
      label: l10n.expiresSemantics((remaining / 60).ceil()),
      excludeSemantics: true,
      child: Text(
        l10n.expiresIn('$m:$s'),
        style: remaining <= 30
            ? AppText.label.copyWith(color: AppColors.error)
            : AppText.label,
      ),
    );
  }
}

/// Code affiché en tuiles, en deux groupes de trois (ex. 123 456).
class _CodeTiles extends StatelessWidget {
  const _CodeTiles({required this.code});

  final String code;

  @override
  Widget build(BuildContext context) {
    final digits = code.split('');
    final groups = digits.length == 6
        ? [digits.sublist(0, 3), digits.sublist(3)]
        : [digits];

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var g = 0; g < groups.length; g++) ...[
          if (g > 0) const SizedBox(width: AppSpacing.md),
          for (var i = 0; i < groups[g].length; i++) ...[
            if (i > 0) const SizedBox(width: AppSpacing.xs),
            _DigitTile(digit: groups[g][i]),
          ],
        ],
      ],
    );
  }
}

class _DigitTile extends StatelessWidget {
  const _DigitTile({required this.digit});

  final String digit;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.sm,
        vertical: AppSpacing.xs,
      ),
      decoration: BoxDecoration(
        color: AppColors.surface,
        borderRadius: BorderRadius.circular(AppRadius.md),
        border: Border.all(color: AppColors.surfaceHigh, width: 2),
      ),
      child: Text(
        digit,
        style: AppText.display.copyWith(
          letterSpacing: 0,
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
      ),
    );
  }
}

/// QR sur fond blanc (zone de silence), encadré par le dégradé de marque.
class _QrCard extends StatelessWidget {
  const _QrCard({required this.data, required this.size});

  final String data;
  final double size;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: AppLocalizations.of(context)!.pairingQrSemantics,
      image: true,
      excludeSemantics: true,
      // RepaintBoundary + clé sur le contenu : le QR est repeint proprement
      // quand la taille de la fenêtre ou le code change.
      child: RepaintBoundary(
        child: Container(
          padding: const EdgeInsets.all(4),
          decoration: BoxDecoration(
            gradient: AppBrand.gradient,
            borderRadius: BorderRadius.circular(AppRadius.lg + 4),
            boxShadow: const [
              BoxShadow(
                color: AppBrand.glow,
                blurRadius: 32,
                offset: Offset(0, 8),
              ),
            ],
          ),
          child: Container(
            padding: const EdgeInsets.all(AppSpacing.md),
            decoration: BoxDecoration(
              color: Colors.white,
              borderRadius: BorderRadius.circular(AppRadius.lg),
            ),
            child: QrImageView(
              key: ValueKey('$data@${size.round()}'),
              data: data,
              size: size,
              padding: EdgeInsets.zero,
              backgroundColor: Colors.white,
              eyeStyle: const QrEyeStyle(
                eyeShape: QrEyeShape.square,
                color: Colors.black,
              ),
              dataModuleStyle: const QrDataModuleStyle(
                dataModuleShape: QrDataModuleShape.square,
                color: Colors.black,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
