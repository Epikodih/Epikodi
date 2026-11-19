import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../theme.dart';

/// Saisie du code à 6 chiffres, en tuiles comme sur la TV (deux groupes de
/// trois). Un vrai champ texte, invisible, reçoit le clavier numérique : le
/// collage, l'effacement et les lecteurs d'écran fonctionnent normalement.
class CodeInput extends StatefulWidget {
  const CodeInput({
    super.key,
    required this.controller,
    required this.onCompleted,
    required this.semanticLabel,
    this.length = 6,
    this.enabled = true,
    this.hasError = false,
    this.focusNode,
  });

  /// Pour donner le focus au champ depuis l'écran (ex. après un scan).
  final FocusNode? focusNode;

  final TextEditingController controller;
  final ValueChanged<String> onCompleted;
  final String semanticLabel;
  final int length;
  final bool enabled;
  final bool hasError;

  @override
  State<CodeInput> createState() => _CodeInputState();
}

class _CodeInputState extends State<CodeInput> {
  late final FocusNode _focus = widget.focusNode ?? FocusNode();

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_changed);
    _focus.addListener(_changed);
  }

  @override
  void didUpdateWidget(CodeInput old) {
    super.didUpdateWidget(old);
    if (old.controller != widget.controller) {
      old.controller.removeListener(_changed);
      widget.controller.addListener(_changed);
    }
    // Le champ perd le focus pendant l'envoi : on le rend après une erreur,
    // pour retaper le code sans toucher l'écran.
    if (!old.enabled && widget.enabled) _focus.requestFocus();
  }

  @override
  void dispose() {
    widget.controller.removeListener(_changed);
    _focus.removeListener(_changed);
    if (widget.focusNode == null) _focus.dispose();
    super.dispose();
  }

  void _changed() {
    if (mounted) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final text = widget.controller.text;
    final focused = _focus.hasFocus;
    final half = widget.length ~/ 2;

    Widget tile(int i) {
      final current =
          focused && i == text.length.clamp(0, widget.length - 1);
      final borderColor = widget.hasError
          ? AppColors.error
          : current
              ? AppColors.primary
              : AppColors.surfaceHigh;
      return Expanded(
        child: AspectRatio(
          aspectRatio: 0.8,
          child: AnimatedContainer(
            duration: AppDurations.fast,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              color: AppColors.surface,
              borderRadius: BorderRadius.circular(AppRadius.md),
              border: Border.all(color: borderColor, width: 2),
            ),
            child: FittedBox(
              fit: BoxFit.scaleDown,
              child: Text(i < text.length ? text[i] : '',
                  style: RemoteText.display),
            ),
          ),
        ),
      );
    }

    return Stack(
      children: [
        ExcludeSemantics(
          child: Row(
            children: [
              for (var i = 0; i < widget.length; i++) ...[
                if (i > 0)
                  SizedBox(width: i == half ? AppSpacing.md : AppSpacing.xs - 2),
                tile(i),
              ],
            ],
          ),
        ),
        Positioned.fill(
          child: Opacity(
            opacity: 0,
            child: TextField(
              key: const ValueKey('code-field'),
              controller: widget.controller,
              focusNode: _focus,
              enabled: widget.enabled,
              keyboardType: TextInputType.number,
              textInputAction: TextInputAction.done,
              autofillHints: const [AutofillHints.oneTimeCode],
              showCursor: false,
              enableInteractiveSelection: false,
              maxLength: widget.length,
              inputFormatters: [FilteringTextInputFormatter.digitsOnly],
              expands: true,
              maxLines: null,
              decoration: InputDecoration(
                counterText: '',
                filled: false,
                border: InputBorder.none,
                enabledBorder: InputBorder.none,
                focusedBorder: InputBorder.none,
                disabledBorder: InputBorder.none,
                hintText: widget.semanticLabel,
              ),
              onChanged: (v) {
                if (v.length == widget.length) widget.onCompleted(v);
              },
            ),
          ),
        ),
      ],
    );
  }
}
