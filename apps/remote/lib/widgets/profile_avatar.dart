import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

/// Pastille ronde dégradée avec l'initiale du profil (comme sur la TV).
/// Sans nom : icône de personne.
class ProfileAvatar extends StatelessWidget {
  const ProfileAvatar({super.key, required this.name, required this.size});

  final String? name;
  final double size;

  @override
  Widget build(BuildContext context) {
    final n = name;
    return Container(
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: const BoxDecoration(
        gradient: AppBrand.gradient,
        shape: BoxShape.circle,
      ),
      child: n == null || n.isEmpty
          ? Icon(Icons.person_rounded, size: size * 0.6, color: Colors.white)
          : Text(
              n.characters.first.toUpperCase(),
              style: TextStyle(
                fontSize: size * 0.45,
                fontWeight: FontWeight.w700,
                color: Colors.white,
              ),
            ),
    );
  }
}
