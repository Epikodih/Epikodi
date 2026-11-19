import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

import '../theme.dart';

/// Bouton principal : pilule au dégradé de marque (comme « Nouveau code »
/// sur la TV). [loading] affiche un indicateur à la place de l'icône.
class BrandButton extends StatelessWidget {
  const BrandButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.icon,
    this.loading = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final bool loading;

  @override
  Widget build(BuildContext context) {
    final enabled = onPressed != null && !loading;
    return Semantics(
      button: true,
      enabled: enabled,
      label: label,
      excludeSemantics: true,
      child: Opacity(
        opacity: enabled || loading ? 1 : 0.5,
        child: DecoratedBox(
          decoration: BoxDecoration(
            gradient: AppBrand.gradient,
            borderRadius: BorderRadius.circular(999),
            boxShadow: enabled
                ? const [
                    BoxShadow(
                        color: AppBrand.glow,
                        blurRadius: 24,
                        offset: Offset(0, 6)),
                  ]
                : null,
          ),
          child: Material(
            type: MaterialType.transparency,
            child: InkWell(
              borderRadius: BorderRadius.circular(999),
              onTap: enabled ? onPressed : null,
              child: _ButtonContent(
                label: label,
                icon: icon,
                loading: loading,
                color: Colors.white,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Bouton secondaire : pilule sombre bordée.
class GhostButton extends StatelessWidget {
  const GhostButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.icon,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      enabled: onPressed != null,
      label: label,
      excludeSemantics: true,
      child: Material(
        color: AppColors.surface,
        shape: const StadiumBorder(
            side: BorderSide(color: AppColors.surfaceHigh, width: 2)),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onPressed,
          child: _ButtonContent(
              label: label, icon: icon, loading: false, color: AppColors.text),
        ),
      ),
    );
  }
}

class _ButtonContent extends StatelessWidget {
  const _ButtonContent({
    required this.label,
    required this.icon,
    required this.loading,
    required this.color,
  });

  final String label;
  final IconData? icon;
  final bool loading;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return ConstrainedBox(
      constraints: const BoxConstraints(minHeight: 56),
      child: Padding(
        padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.md, vertical: AppSpacing.sm - 4),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            if (loading)
              SizedBox.square(
                dimension: 22,
                child: CircularProgressIndicator(strokeWidth: 3, color: color),
              )
            else if (icon != null)
              Icon(icon, size: 24, color: color),
            if (loading || icon != null)
              const SizedBox(width: AppSpacing.xs + 4),
            Flexible(
              child: Text(
                label,
                textAlign: TextAlign.center,
                style: RemoteText.title.copyWith(fontSize: 17, color: color),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
