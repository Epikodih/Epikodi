import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

/// Bouton pilule avec du texte (et une icône optionnelle).
class PillButton extends StatelessWidget {
  const PillButton(
      {super.key, required this.text, required this.onTap, this.icon});

  final String text;
  final VoidCallback onTap;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final iconSize = MediaQuery.textScalerOf(context).scale(28);
    return FocusableCard(
      borderRadius: 999,
      onTap: onTap,
      semanticLabel: text,
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.lg,
        vertical: AppSpacing.sm,
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: iconSize, color: AppColors.text),
            const SizedBox(width: AppSpacing.xs),
          ],
          Text(text, style: AppText.body.copyWith(fontWeight: FontWeight.w600)),
        ],
      ),
    );
  }
}
