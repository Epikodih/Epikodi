import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'tokens.dart';

/// Carte navigable à la télécommande : contour blanc épais + léger zoom au focus.
/// OK / Entrée / bouton A déclenchent [onTap].
/// Respecte « animations désactivées » (MediaQuery.disableAnimations).
class FocusableCard extends StatefulWidget {
  const FocusableCard({
    super.key,
    required this.child,
    this.onTap,
    this.autofocus = false,
    this.focusNode,
    this.borderRadius = AppRadius.md,
    this.padding = EdgeInsets.zero,
    this.semanticLabel,
  });

  final Widget child;
  final VoidCallback? onTap;
  final bool autofocus;
  final FocusNode? focusNode;
  final double borderRadius;
  final EdgeInsetsGeometry padding;

  /// Texte lu par le lecteur d'écran (sinon : le texte de l'enfant).
  final String? semanticLabel;

  @override
  State<FocusableCard> createState() => _FocusableCardState();
}

class _FocusableCardState extends State<FocusableCard> {
  bool _focused = false;

  static final _selectKeys = {
    LogicalKeyboardKey.select,
    LogicalKeyboardKey.enter,
    LogicalKeyboardKey.numpadEnter,
    LogicalKeyboardKey.gameButtonA,
  };

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is KeyDownEvent && _selectKeys.contains(event.logicalKey)) {
      widget.onTap?.call();
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  void _onFocusChange(bool focused) {
    setState(() => _focused = focused);
    if (focused) {
      final reduce = MediaQuery.disableAnimationsOf(context);
      Scrollable.ensureVisible(
        context,
        duration: reduce ? Duration.zero : AppDurations.fast,
        alignment: 0.5,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final reduce = MediaQuery.disableAnimationsOf(context);
    final dur = reduce ? Duration.zero : AppDurations.fast;
    final radius = BorderRadius.circular(widget.borderRadius);

    return Semantics(
      button: widget.onTap != null,
      focusable: true,
      focused: _focused,
      label: widget.semanticLabel,
      onTap: widget.onTap,
      child: Focus(
        autofocus: widget.autofocus,
        focusNode: widget.focusNode,
        onFocusChange: _onFocusChange,
        onKeyEvent: _onKey,
        child: GestureDetector(
          onTap: widget.onTap,
          child: AnimatedScale(
            scale: (_focused && !reduce) ? AppFocus.scale : 1.0,
            duration: dur,
            child: AnimatedContainer(
              duration: dur,
              padding: widget.padding,
              decoration: BoxDecoration(
                color: _focused ? AppColors.surfaceHigh : AppColors.surface,
                borderRadius: radius,
                border: Border.all(
                  color: _focused ? AppColors.focus : Colors.transparent,
                  width: AppFocus.borderWidth,
                ),
                boxShadow: _focused
                    ? const [BoxShadow(color: Color(0x66000000), blurRadius: 24, offset: Offset(0, 8))]
                    : null,
              ),
              child: ClipRRect(borderRadius: radius, child: widget.child),
            ),
          ),
        ),
      ),
    );
  }
}