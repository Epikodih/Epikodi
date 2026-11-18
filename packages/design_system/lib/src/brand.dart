import 'package:flutter/material.dart';

import 'gloubi.dart';
import 'tokens.dart';

/// Identité visuelle GloubiTV : dégradé bleu → violet.
/// Les couleurs de base (fond, texte, focus) restent dans [AppColors].
abstract final class AppBrand {
  static const violet = Color(0xFF8B6CFF);

  static const gradient = LinearGradient(
    begin: Alignment.topLeft,
    end: Alignment.bottomRight,
    colors: [AppColors.primary, violet],
  );

  /// Halo violet semi-transparent (~35 %) pour les ombres colorées.
  static const glow = Color(0x598B6CFF);
}

/// Logo complet : Gloubi (image fixe) + mot-symbole « Gloubi » (léger) « TV » (gras, dégradé).
class GloubiLogo extends StatelessWidget {
  const GloubiLogo({super.key, this.size = 72, this.showWordmark = true});

  final double size;
  final bool showWordmark;

  @override
  Widget build(BuildContext context) {
    final fontSize = size * 0.72;
    return Semantics(
      label: 'GloubiTV',
      header: true,
      excludeSemantics: true,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          // Le widget Gloubi existant, en image fixe.
          Gloubi(size: size),
          if (showWordmark) ...[
            SizedBox(width: size * 0.2),
            Text.rich(
              TextSpan(children: [
                TextSpan(
                  text: 'Gloubi',
                  style: TextStyle(
                    fontSize: fontSize,
                    fontWeight: FontWeight.w300,
                    color: AppColors.text,
                    letterSpacing: -1,
                  ),
                ),
                WidgetSpan(
                  alignment: PlaceholderAlignment.baseline,
                  baseline: TextBaseline.alphabetic,
                  child: ShaderMask(
                    blendMode: BlendMode.srcIn,
                    shaderCallback: (r) => AppBrand.gradient.createShader(r),
                    child: Text(
                      'TV',
                      style: TextStyle(
                        fontSize: fontSize,
                        fontWeight: FontWeight.w800,
                        color: Colors.white,
                        letterSpacing: -1,
                      ),
                    ),
                  ),
                ),
              ]),
            ),
          ],
        ],
      ),
    );
  }
}

/// Fond de marque : couleur de fond + deux halos diffus (bleu et violet).
/// Statique : rien à désactiver pour « animations réduites ».
class BrandBackground extends StatelessWidget {
  const BrandBackground({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Stack(
      fit: StackFit.expand,
      children: [
        const ColoredBox(color: AppColors.background),
        Positioned.fill(
          child: DecoratedBox(
            decoration: BoxDecoration(
              gradient: RadialGradient(
                center: const Alignment(-1.0, -1.0),
                radius: 1.1,
                colors: [AppColors.primary.withAlpha(46), Colors.transparent],
              ),
            ),
          ),
        ),
        Positioned.fill(
          child: DecoratedBox(
            decoration: BoxDecoration(
              gradient: RadialGradient(
                center: const Alignment(1.0, 1.0),
                radius: 1.0,
                colors: [AppBrand.violet.withAlpha(38), Colors.transparent],
              ),
            ),
          ),
        ),
        child,
      ],
    );
  }
}
