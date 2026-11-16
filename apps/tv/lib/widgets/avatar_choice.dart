import 'package:epikodi_design_system/epikodi_design_system.dart';
import 'package:flutter/material.dart';

/// Pastille d'avatar à choisir (coche + anneau quand sélectionnée).
class AvatarChoice extends StatelessWidget {
  const AvatarChoice({
    super.key,
    required this.color,
    required this.selected,
    required this.semantic,
    required this.onTap,
  });

  final Color color;
  final bool selected;
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
        child: Center(
          child: Container(
            width: size * 0.78,
            height: size * 0.78,
            decoration: BoxDecoration(
              color: color,
              shape: BoxShape.circle,
              border:
                  selected ? Border.all(color: Colors.white, width: 3) : null,
            ),
            child: selected
                ? Icon(Icons.check_rounded,
                    size: size * 0.45, color: Colors.white)
                : null,
          ),
        ),
      ),
    );
  }
}
