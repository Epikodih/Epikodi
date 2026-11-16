import 'package:epikodi_design_system/epikodi_design_system.dart';
import 'package:flutter/material.dart';

/// Bouton rond avec une icône (− / +).
class RoundButton extends StatelessWidget {
  const RoundButton({
    super.key,
    required this.icon,
    required this.semantic,
    required this.onTap,
  });

  final IconData icon;
  final String semantic;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final size = MediaQuery.textScalerOf(context).scale(56);
    return SizedBox(
      width: size,
      height: size,
      child: FocusableCard(
        onTap: onTap,
        borderRadius: 999,
        padding: EdgeInsets.zero,
        semanticLabel: semantic,
        child:
            Center(child: Icon(icon, size: size * 0.5, color: AppColors.text)),
      ),
    );
  }
}
