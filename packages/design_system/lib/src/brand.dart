import 'package:flutter/material.dart';

import 'tokens.dart';

/// Identité visuelle EpiKodi : dégradé bleu → violet, accent cyan.
/// Les couleurs de base (fond, texte, focus) restent dans [AppColors].
abstract final class AppBrand {
  static const violet = Color(0xFF8B6CFF);
  static const accent = Color(0xFF35D4E8);

  static const gradient = LinearGradient(
    begin: Alignment.topLeft,
    end: Alignment.bottomRight,
    colors: [AppColors.primary, violet],
  );

  /// Halo violet semi-transparent (~35 %) pour les ombres colorées.
  static const glow = Color(0x598B6CFF);
}

/// Pastille de marque : carré arrondi dégradé + symbole lecture.
class EpikodiMark extends StatelessWidget {
  const EpikodiMark({super.key, this.size = 72});

  final double size;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        gradient: AppBrand.gradient,
        borderRadius: BorderRadius.circular(size * 0.3),
        boxShadow: const [BoxShadow(color: AppBrand.glow, blurRadius: 32, offset: Offset(0, 8))],
      ),
      child: Icon(Icons.play_arrow_rounded, size: size * 0.7, color: Colors.white),
    );
  }
}

/// Logo complet : pastille + mot-symbole « Epi » (léger) « Kodi » (gras, dégradé).
class EpikodiLogo extends StatelessWidget {
  const EpikodiLogo({super.key, this.size = 72, this.showWordmark = true});

  final double size;
  final bool showWordmark;

  @override
  Widget build(BuildContext context) {
    final fontSize = size * 0.72;
    return Semantics(
      label: 'EpiKodi',
      header: true,
      excludeSemantics: true,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          EpikodiMark(size: size),
          if (showWordmark) ...[
            SizedBox(width: size * 0.3),
            Text.rich(
              TextSpan(children: [
                TextSpan(
                  text: 'Epi',
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
                      'Kodi',
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