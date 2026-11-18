import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// Taille des sous-titres (multiplicateur de la taille de base).
enum SubtitleSize {
  small(0.75),
  normal(1.0),
  large(1.35),
  extraLarge(1.75);

  const SubtitleSize(this.factor);
  final double factor;
}

/// Couleur du texte. Toutes gardent un contraste supérieur à 4,5:1 sur noir.
enum SubtitleColor {
  white(Color(0xFFFFFFFF)),
  yellow(Color(0xFFFFE14D)),
  cyan(Color(0xFF7FE9FF));

  const SubtitleColor(this.color);
  final Color color;
}

/// Fond derrière le texte.
enum SubtitleBackground {
  none(Color(0x00000000)),
  translucent(Color(0xB3000000)),
  opaque(Color(0xFF000000));

  const SubtitleBackground(this.color);
  final Color color;
}

/// Apparence des sous-titres, réglable par l'utilisateur (#22).
@immutable
class SubtitleStyle {
  const SubtitleStyle({
    this.size = SubtitleSize.normal,
    this.color = SubtitleColor.white,
    this.background = SubtitleBackground.translucent,
  });

  /// Taille de base à 3 m d'un téléviseur, avant [SubtitleSize.factor] et
  /// avant la taille de texte globale de l'app.
  static const baseFontSize = 44.0;

  final SubtitleSize size;
  final SubtitleColor color;
  final SubtitleBackground background;

  SubtitleStyle copyWith({
    SubtitleSize? size,
    SubtitleColor? color,
    SubtitleBackground? background,
  }) =>
      SubtitleStyle(
        size: size ?? this.size,
        color: color ?? this.color,
        background: background ?? this.background,
      );

  /// Style de texte appliqué par le moteur.
  /// Sans fond, un contour sombre garde le texte lisible sur une image claire.
  TextStyle toTextStyle() => TextStyle(
        height: 1.4,
        fontSize: baseFontSize * size.factor,
        fontWeight: FontWeight.w600,
        color: color.color,
        backgroundColor: background.color,
        shadows: background == SubtitleBackground.none
            ? const [
                Shadow(color: Color(0xFF000000), blurRadius: 4),
                Shadow(color: Color(0xFF000000), offset: Offset(2, 2)),
              ]
            : null,
      );

  @override
  bool operator ==(Object other) =>
      other is SubtitleStyle &&
      other.size == size &&
      other.color == color &&
      other.background == background;

  @override
  int get hashCode => Object.hash(size, color, background);
}
