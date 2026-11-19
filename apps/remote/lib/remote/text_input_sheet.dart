import 'package:design_system/design_system.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme.dart';
import '../widgets/buttons.dart';
import 'remote_events.dart';

/// Clavier du téléphone pour le champ texte qui a le focus sur la TV.
///
/// Chaque lettre tapée est envoyée tout de suite ([onChanged]) : on voit le
/// texte s'écrire sur la TV. « Valider » ([onDone]) fait comme Entrée.
/// La feuille se ferme seule quand la TV quitte le champ ([request] passe à
/// `null`), et repart de zéro si la TV passe à un autre champ.
Future<void> showTextInputSheet(
  BuildContext context, {
  required ValueListenable<TextInputRequest?> request,
  required ValueChanged<String> onChanged,
  required ValueChanged<String> onDone,
}) =>
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      backgroundColor: AppColors.surface,
      shape: const RoundedRectangleBorder(
          borderRadius:
              BorderRadius.vertical(top: Radius.circular(AppRadius.lg))),
      builder: (_) => _TextInputSheet(
          request: request, onChanged: onChanged, onDone: onDone),
    );

class _TextInputSheet extends StatefulWidget {
  const _TextInputSheet({
    required this.request,
    required this.onChanged,
    required this.onDone,
  });

  final ValueListenable<TextInputRequest?> request;
  final ValueChanged<String> onChanged;
  final ValueChanged<String> onDone;

  @override
  State<_TextInputSheet> createState() => _TextInputSheetState();
}

class _TextInputSheetState extends State<_TextInputSheet> {
  final _text = TextEditingController();
  TextInputRequest? _current;
  bool _closing = false;

  @override
  void initState() {
    super.initState();
    _apply(widget.request.value);
    widget.request.addListener(_onRequest);
  }

  @override
  void dispose() {
    widget.request.removeListener(_onRequest);
    _text.dispose();
    super.dispose();
  }

  void _onRequest() {
    final next = widget.request.value;
    if (next == null) {
      _close();
      return;
    }
    if (next.id != _current?.id) setState(() => _apply(next));
  }

  void _apply(TextInputRequest? r) {
    _current = r;
    _text.value = TextEditingValue(
      text: r?.value ?? '',
      selection: TextSelection.collapsed(offset: r?.value.length ?? 0),
    );
  }

  void _close() {
    if (_closing || !mounted) return;
    _closing = true;
    Navigator.of(context).pop();
  }

  void _done() {
    widget.onDone(_text.text);
    _close();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final r = _current;
    final title = (r == null || r.label.isEmpty) ? l10n.keyboardTitle : r.label;
    return Padding(
      padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(context).bottom),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(
            AppSpacing.md, AppSpacing.sm, AppSpacing.md, AppSpacing.md),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                const Icon(Icons.keyboard_rounded, color: AppColors.primary),
                const SizedBox(width: AppSpacing.xs),
                Expanded(
                  child: Text(title,
                      style: RemoteText.title, overflow: TextOverflow.ellipsis),
                ),
                IconButton(
                  tooltip: l10n.keyboardClose,
                  onPressed: _close,
                  icon: const Icon(Icons.close_rounded, color: AppColors.text),
                ),
              ],
            ),
            const SizedBox(height: AppSpacing.sm),
            TextField(
              key: const ValueKey('tv-text-field'),
              controller: _text,
              autofocus: true,
              obscureText: r?.obscure ?? false,
              keyboardType:
                  (r?.number ?? false) ? TextInputType.number : TextInputType.text,
              maxLength: r?.maxLength,
              textInputAction: TextInputAction.done,
              decoration: InputDecoration(
                hintText: l10n.keyboardTitle,
                prefixIcon: const Icon(Icons.tv_rounded),
              ),
              onChanged: widget.onChanged,
              onSubmitted: (_) => _done(),
            ),
            const SizedBox(height: AppSpacing.sm),
            BrandButton(
              label: l10n.keyboardDone,
              icon: Icons.check_rounded,
              onPressed: _done,
            ),
          ],
        ),
      ),
    );
  }
}
