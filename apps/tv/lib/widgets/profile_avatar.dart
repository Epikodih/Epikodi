import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

/// Pastille ronde dégradée avec l'initiale du profil.
/// Réutilisable (menu du profil, page profil…).
class ProfileAvatar extends StatelessWidget {
  const ProfileAvatar({super.key, required this.name, required this.size});

  final String name;
  final double size;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: const BoxDecoration(
        gradient: AppBrand.gradient,
        shape: BoxShape.circle,
      ),
      child: FittedBox(
        child: Padding(
          padding: EdgeInsets.all(size * 0.2),
          child: Text(
            name.isEmpty ? '?' : name.characters.first.toUpperCase(),
            style: AppText.headline.copyWith(color: Colors.white),
          ),
        ),
      ),
    );
  }
}
