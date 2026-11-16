import 'dart:io';

import 'package:epikodi_design_system/epikodi_design_system.dart';
import 'package:flutter/material.dart';

/// Grand aperçu de l'avatar choisi (initiale du nom, ou icône par défaut).
class AvatarPreview extends StatelessWidget {
  const AvatarPreview({
    super.key,
    required this.color,
    required this.initial,
    required this.size,
    this.imagePath,
  });

  final Color color;
  final String? initial;
  final double size;
  final String? imagePath;

  @override
  Widget build(BuildContext context) {
    if (imagePath != null) {
      return ClipOval(
        child: Image.file(
          File(imagePath!),
          width: size,
          height: size,
          fit: BoxFit.cover,
          errorBuilder: (_, __, ___) => Container(
            width: size,
            height: size,
            color: color,
            child: Icon(Icons.broken_image_rounded,
                size: size * 0.5, color: Colors.white),
          ),
        ),
      );
    }
    return Container(
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: BoxDecoration(color: color, shape: BoxShape.circle),
      child: initial == null
          ? Icon(Icons.person_rounded, size: size * 0.55, color: Colors.white)
          : Text(
              initial!,
              style: AppText.headline.copyWith(
                color: Colors.white,
                fontSize: size * 0.45,
              ),
            ),
    );
  }
}
