import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Nombre de chiffres du code PIN.
const pinLength = 4;

/// Popup « Code PIN » : pavé numérique navigable avec les flèches + Entrée
/// (le clavier physique marche aussi : chiffres, Retour arrière, Échap).
///
/// Utilisation :
/// ```dart
/// await showDialog(
///   context: context,
///   builder: (_) => PinDialog(
///     profileName: 'Marc',
///     onSubmit: (pin) async { ... return null; /* ok */ },
///   ),
/// );
/// ```
class PinDialog extends StatefulWidget {
  const PinDialog({
    super.key,
    required this.profileName,
    required this.onSubmit,
  });

  final String profileName;

  /// Appelée quand les [pinLength] chiffres sont saisis.
  /// Renvoie `null` si le code est bon (la popup se ferme),
  /// sinon le message d'erreur à afficher (la saisie est effacée).
  final Future<String?> Function(String pin) onSubmit;

  @override
  State<PinDialog> createState() => _PinDialogState();
}

class _PinDialogState extends State<PinDialog> {
  static const _errorColor = Color(0xFFFF8A80);
  // Même rouge, ~15 % d'opacité : fond de l'encart d'erreur.
  static const _errorBackground = Color(0x26FF8A80);

  String _pin = '';
  String? _error;
  bool _busy = false;

  void _digit(String d) {
    if (_busy || _pin.length >= pinLength) return;
    setState(() {
      _pin += d;
      _error = null;
    });
    if (_pin.length == pinLength) _submit();
  }

  void _erase() {
    if (_busy || _pin.isEmpty) return;
    setState(() {
      _pin = _pin.substring(0, _pin.length - 1);
      _error = null;
    });
  }

  Future<void> _submit() async {
    setState(() => _busy = true);
    final error = await widget.onSubmit(_pin);
    if (!mounted) return;
    if (error == null) {
      Navigator.of(context).pop();
      return;
    }
    setState(() {
      _busy = false;
      _pin = '';
      _error = error;
    });
  }

  /// Clavier physique : chiffres et Retour arrière.
  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent) return KeyEventResult.ignored;
    if (event.logicalKey == LogicalKeyboardKey.backspace) {
      _erase();
      return KeyEventResult.handled;
    }
    final c = event.character;
    if (c != null && c.length == 1) {
      final code = c.codeUnitAt(0);
      if (code >= 48 && code <= 57) {
        _digit(c);
        return KeyEventResult.handled;
      }
    }
    return KeyEventResult.ignored;
  }

  /// Rangée de points : rempli = chiffre saisi, rouge = dernier essai refusé.
  Widget _buildDots() {
    final hasError = _error != null;
    return Semantics(
      label: '${_pin.length} chiffres saisis sur $pinLength',
      child: ExcludeSemantics(
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            for (var i = 0; i < pinLength; i++)
              AnimatedContainer(
                duration: const Duration(milliseconds: 120),
                width: 22,
                height: 22,
                margin: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: i < _pin.length ? AppColors.text : Colors.transparent,
                  border: Border.all(
                    color: hasError ? _errorColor : AppColors.text,
                    width: 2,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  /// Encart d'erreur : pleine largeur, texte centré, jusqu'à 3 lignes.
  /// La zone garde une hauteur minimale pour que la popup ne « saute » pas.
  Widget _buildErrorZone(TextScaler scaler) {
    return ConstrainedBox(
      constraints: BoxConstraints(minHeight: scaler.scale(84)),
      child: Center(
        child: AnimatedSwitcher(
          duration: const Duration(milliseconds: 150),
          child: _error == null
              ? const SizedBox.shrink(key: ValueKey('no-error'))
              : Container(
                  key: ValueKey(_error),
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(
                    horizontal: AppSpacing.md,
                    vertical: AppSpacing.sm,
                  ),
                  decoration: BoxDecoration(
                    color: _errorBackground,
                    borderRadius: BorderRadius.circular(AppRadius.lg),
                    border: Border.all(color: _errorColor, width: 1.5),
                  ),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      const Icon(Icons.error_outline_rounded,
                          color: _errorColor),
                      const SizedBox(width: AppSpacing.sm),
                      Flexible(
                        child: Text(
                          _error!,
                          style: AppText.body.copyWith(color: _errorColor),
                          textAlign: TextAlign.center,
                          maxLines: 3,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final scaler = MediaQuery.textScalerOf(context);
    final keySize = scaler.scale(76);
    const gap = AppSpacing.sm;
    final padWidth = 3 * keySize + 2 * gap;
    // Contenu plus large que le pavé : le message d'erreur tient sans
    // être coupé. Le pavé reste centré dedans.
    final contentWidth =
        padWidth > scaler.scale(400) ? padWidth : scaler.scale(400);

    Widget key({
      String? label,
      IconData? icon,
      required VoidCallback onTap,
      required String semantic,
      bool autofocus = false,
    }) {
      return SizedBox(
        width: keySize,
        height: keySize,
        child: FocusableCard(
          autofocus: autofocus,
          onTap: onTap,
          borderRadius: 999,
          padding: EdgeInsets.zero,
          semanticLabel: semantic,
          child: Center(
            child: label != null
                ? Text(label, style: AppText.title)
                : Icon(icon, size: keySize * 0.42, color: AppColors.text),
          ),
        ),
      );
    }

    Widget digitRow(List<String> digits) => Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            for (final d in digits)
              key(
                label: d,
                semantic: d,
                autofocus: d == '1',
                onTap: () => _digit(d),
              ),
          ],
        );

    return Focus(
      canRequestFocus: false,
      skipTraversal: true,
      onKeyEvent: _onKey,
      child: Dialog(
        backgroundColor: AppColors.surfaceHigh,
        insetPadding: const EdgeInsets.all(AppSpacing.md),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppRadius.lg),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.lg,
            vertical: AppSpacing.lg,
          ),
          // Si la fenêtre est petite, la popup se réduit au lieu de déborder.
          child: FittedBox(
            fit: BoxFit.scaleDown,
            child: SizedBox(
              width: contentWidth,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  // --- En-tête : cadenas + titre + nom du profil
                  Container(
                    padding: const EdgeInsets.all(AppSpacing.sm),
                    decoration: const BoxDecoration(
                      shape: BoxShape.circle,
                      color: Color(0x1FFFFFFF),
                    ),
                    child: Icon(Icons.lock_rounded,
                        size: 28, color: AppColors.text),
                  ),
                  const SizedBox(height: AppSpacing.sm),
                  Text('Code PIN', style: AppText.headline),
                  const SizedBox(height: AppSpacing.xs),
                  Text(
                    'pour ${widget.profileName}',
                    style: AppText.body.copyWith(color: AppColors.textMuted),
                    textAlign: TextAlign.center,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: AppSpacing.md),

                  _buildDots(),
                  const SizedBox(height: AppSpacing.sm),
                  _buildErrorZone(scaler),
                  const SizedBox(height: AppSpacing.sm),

                  // --- Pavé numérique, centré
                  SizedBox(
                    width: padWidth,
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        digitRow(const ['1', '2', '3']),
                        const SizedBox(height: gap),
                        digitRow(const ['4', '5', '6']),
                        const SizedBox(height: gap),
                        digitRow(const ['7', '8', '9']),
                        const SizedBox(height: gap),
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            key(
                              icon: Icons.backspace_outlined,
                              semantic: 'Effacer le dernier chiffre',
                              onTap: _erase,
                            ),
                            key(
                              label: '0',
                              semantic: '0',
                              onTap: () => _digit('0'),
                            ),
                            key(
                              icon: Icons.close_rounded,
                              semantic: 'Annuler',
                              onTap: () => Navigator.of(context).pop(),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
