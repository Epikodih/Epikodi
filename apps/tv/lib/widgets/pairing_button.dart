import 'package:epikodi_design_system/epikodi_design_system.dart';
import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import 'pill_metrics.dart';

/// Bouton « Appairage » (pilule discrète) : retour à l'écran d'appairage.
/// Réutilisable sur n'importe quel écran. Hauteur identique au sélecteur
/// de langue ([pillButtonHeight]).
class PairingButton extends StatelessWidget {
  const PairingButton({super.key, required this.onTap, this.autofocus = false});

  final VoidCallback onTap;
  final bool autofocus;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final scaler = MediaQuery.textScalerOf(context);
    return FocusableCard(
      autofocus: autofocus,
      borderRadius: 999,
      onTap: onTap,
      semanticLabel: l10n.backToPairingLabel,
      // Padding horizontal seulement : la hauteur est fixée par le SizedBox.
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.md),
      child: SizedBox(
        height: scaler.scale(pillButtonHeight),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Icon(
              Icons.qr_code_2_rounded,
              size: scaler.scale(32),
              color: AppColors.text,
            ),
            const SizedBox(width: AppSpacing.xs + 4),
            Text(
              l10n.pairingButton,
              style: AppText.body.copyWith(
                fontWeight: FontWeight.w600,
                height: 1.3, // évite que le « g » soit coupé
              ),
            ),
          ],
        ),
      ),
    );
  }
}
