import 'package:epikodi_design_system/epikodi_design_system.dart';
import 'package:flutter/material.dart';

/// Choix avec case cochée / vide : l'état est montré par l'icône, pas par
/// la seule couleur.
class PillChoice extends StatelessWidget {
  const PillChoice({
    super.key,
    required this.text,
    required this.selected,
    required this.onTap,
    this.semanticPrefix,
    this.compact = false,
  });

  final String text;
  final bool selected;
  final VoidCallback onTap;
  final String? semanticPrefix;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final iconSize = MediaQuery.textScalerOf(context).scale(compact ? 22 : 28);
    final base = semanticPrefix == null ? text : '$semanticPrefix : $text';
    return FocusableCard(
      borderRadius: 999,
      onTap: onTap,
      semanticLabel: '$base, ${selected ? 'sélectionné' : 'non sélectionné'}',
      padding: EdgeInsets.symmetric(
        horizontal: compact ? AppSpacing.sm : AppSpacing.md,
        vertical: compact ? AppSpacing.xs : AppSpacing.sm,
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            selected
                ? Icons.check_circle_rounded
                : Icons.radio_button_unchecked_rounded,
            size: iconSize,
            color: AppColors.text,
          ),
          const SizedBox(width: AppSpacing.xs),
          Text(text, style: AppText.body),
        ],
      ),
    );
  }
}
