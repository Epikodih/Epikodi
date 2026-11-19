import 'dart:math' as math;

import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import 'tv_commands.dart';

/// Pavé directionnel : une seule surface bordée du dégradé de marque,
/// découpée en quatre secteurs (haut, bas, gauche, droite) autour d'un OK
/// central. Le secteur touché s'allume. On peut aussi glisser le doigt sur
/// le pavé dans une direction. Chaque appui vibre légèrement.
class DPad extends StatefulWidget {
  const DPad({super.key, required this.size, required this.onKey});

  final double size;
  final ValueChanged<TvKey> onKey;

  /// Côté du OK, en fraction du pavé.
  static const okFraction = 0.36;

  @override
  State<DPad> createState() => _DPadState();
}

class _DPadState extends State<DPad> {
  TvKey? _pressed;

  void _press(TvKey key) {
    HapticFeedback.selectionClick();
    widget.onKey(key);
  }

  void _highlight(TvKey? key) {
    if (_pressed != key) setState(() => _pressed = key);
  }

  /// Glissement : la direction dominante du geste, s'il est assez rapide.
  void _onSwipe(DragEndDetails d) {
    final v = d.velocity.pixelsPerSecond;
    if (v.distance < 300) return;
    if (v.dx.abs() > v.dy.abs()) {
      _press(v.dx > 0 ? TvKey.right : TvKey.left);
    } else {
      _press(v.dy > 0 ? TvKey.down : TvKey.up);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final s = widget.size;
    final ok = s * DPad.okFraction;
    final band = (s - ok) / 2; // épaisseur d'un secteur

    /// Zone tactile d'un secteur, invisible : le dessin est fait à part.
    Widget zone(TvKey key, String label, Rect r) => Positioned.fromRect(
          rect: r,
          child: Semantics(
            button: true,
            label: label,
            excludeSemantics: true,
            child: GestureDetector(
              key: ValueKey('key-${key.name}'),
              behavior: HitTestBehavior.opaque,
              onTapDown: (_) => _highlight(key),
              onTapCancel: () => _highlight(null),
              onTapUp: (_) {
                _highlight(null);
                _press(key);
              },
            ),
          ),
        );

    return GestureDetector(
      onPanEnd: _onSwipe,
      child: Container(
        width: s,
        height: s,
        padding: const EdgeInsets.all(2),
        decoration: BoxDecoration(
          gradient: AppBrand.gradient,
          borderRadius: BorderRadius.circular(s * 0.16),
          boxShadow: const [
            BoxShadow(color: Colors.black45, blurRadius: 28, offset: Offset(0, 12)),
          ],
        ),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(s * 0.16 - 2),
          child: Stack(
            children: [
              Positioned.fill(
                child: CustomPaint(
                  painter: _PadPainter(okFraction: DPad.okFraction, pressed: _pressed),
                ),
              ),
              // Flèches des secteurs.
              for (final (key, icon, align) in const [
                (TvKey.up, Icons.keyboard_arrow_up_rounded, Alignment(0, -0.78)),
                (TvKey.down, Icons.keyboard_arrow_down_rounded, Alignment(0, 0.78)),
                (TvKey.left, Icons.keyboard_arrow_left_rounded, Alignment(-0.78, 0)),
                (TvKey.right, Icons.keyboard_arrow_right_rounded, Alignment(0.78, 0)),
              ])
                Align(
                  alignment: align,
                  child: IgnorePointer(
                    child: Icon(
                      icon,
                      size: s * 0.15,
                      color: _pressed == key ? Colors.white : AppColors.textMuted,
                    ),
                  ),
                ),
              zone(TvKey.up, l10n.keyUp, Rect.fromLTWH(0, 0, s, band)),
              zone(TvKey.down, l10n.keyDown, Rect.fromLTWH(0, s - band, s, band)),
              zone(TvKey.left, l10n.keyLeft, Rect.fromLTWH(0, band, band, ok)),
              zone(TvKey.right, l10n.keyRight,
                  Rect.fromLTWH(s - band, band, band, ok)),
              // OK au centre.
              Center(
                child: _OkButton(
                  size: ok,
                  label: l10n.keyOk,
                  onTap: () => _press(TvKey.ok),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Fond du pavé : surface sombre, quatre secteurs séparés par des
/// diagonales, le secteur appuyé légèrement éclairé.
class _PadPainter extends CustomPainter {
  _PadPainter({required this.okFraction, required this.pressed});

  final double okFraction;
  final TvKey? pressed;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    canvas.drawRect(
      rect,
      Paint()
        ..shader = const RadialGradient(
          center: Alignment(0, -0.3),
          radius: 0.9,
          colors: [AppColors.surfaceHigh, AppColors.surface],
        ).createShader(rect),
    );

    final c = size.center(Offset.zero);
    final half = size.width * okFraction / 2 + 6;
    final inner = Rect.fromCenter(center: c, width: half * 2, height: half * 2);
    final corners = {
      TvKey.up: [rect.topLeft, rect.topRight, inner.topRight, inner.topLeft],
      TvKey.right: [rect.topRight, rect.bottomRight, inner.bottomRight, inner.topRight],
      TvKey.down: [rect.bottomRight, rect.bottomLeft, inner.bottomLeft, inner.bottomRight],
      TvKey.left: [rect.bottomLeft, rect.topLeft, inner.topLeft, inner.bottomLeft],
    };

    final p = pressed;
    if (p != null && corners[p] != null) {
      final pts = corners[p]!;
      canvas.drawPath(
        Path()..addPolygon(pts, true),
        Paint()..color = AppBrand.violet.withAlpha(60),
      );
    }

    // Diagonales entre les secteurs (des coins vers le OK).
    final line = Paint()
      ..color = Colors.white.withAlpha(22)
      ..strokeWidth = 1.5;
    for (final (a, b) in [
      (rect.topLeft, inner.topLeft),
      (rect.topRight, inner.topRight),
      (rect.bottomLeft, inner.bottomLeft),
      (rect.bottomRight, inner.bottomRight),
    ]) {
      final d = b - a;
      // Ne commence pas tout au bord (coins arrondis).
      final start = a + d * (math.min(0.25, 24 / d.distance));
      canvas.drawLine(start, b, line);
    }
  }

  @override
  bool shouldRepaint(_PadPainter old) =>
      old.pressed != pressed || old.okFraction != okFraction;
}

class _OkButton extends StatelessWidget {
  const _OkButton({required this.size, required this.label, required this.onTap});

  final double size;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final radius = BorderRadius.circular(size * 0.3);
    return Semantics(
      button: true,
      label: label,
      excludeSemantics: true,
      child: DecoratedBox(
        decoration: BoxDecoration(
          gradient: AppBrand.gradient,
          borderRadius: radius,
          boxShadow: const [
            BoxShadow(color: AppBrand.glow, blurRadius: 24, offset: Offset(0, 6)),
          ],
        ),
        child: Material(
          type: MaterialType.transparency,
          borderRadius: radius,
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            key: const ValueKey('key-ok'),
            onTap: onTap,
            splashColor: Colors.white24,
            child: SizedBox.square(
              dimension: size,
              child: Center(
                child: Text(
                  label,
                  style: TextStyle(
                    fontSize: size * 0.26,
                    fontWeight: FontWeight.w800,
                    color: Colors.white,
                    letterSpacing: 1,
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Retour et Accueil réunis dans une seule barre en deux moitiés.
class NavKeys extends StatelessWidget {
  const NavKeys({super.key, required this.onKey});

  final ValueChanged<TvKey> onKey;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;

    Widget half(TvKey key, IconData icon, String label) => Expanded(
          child: Semantics(
            button: true,
            label: label,
            excludeSemantics: true,
            child: InkWell(
              key: ValueKey('key-${key.name}'),
              onTap: () {
                HapticFeedback.selectionClick();
                onKey(key);
              },
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(icon, size: 22, color: AppColors.text),
                  const SizedBox(width: AppSpacing.xs),
                  Flexible(
                    child: Text(
                      label,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                        color: AppColors.text,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );

    return Container(
      height: 58,
      padding: const EdgeInsets.all(2),
      decoration: BoxDecoration(
        gradient: AppBrand.gradient,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Material(
        color: AppColors.surface,
        shape: const StadiumBorder(),
        clipBehavior: Clip.antiAlias,
        child: Row(
          children: [
            half(TvKey.back, Icons.undo_rounded, l10n.keyBack),
            Container(width: 1.5, height: 26, color: AppColors.surfaceHigh),
            half(TvKey.home, Icons.home_outlined, l10n.keyHome),
          ],
        ),
      ),
    );
  }
}
