import 'package:flutter/widgets.dart';

/// Réglages d'affichage valables pour tout le projet.
/// Pensés pour l'accessibilité : taille du texte, taille du QR, animations.
class UiSettings extends ChangeNotifier {
  UiSettings({
    double textScale = 1.0,
    double qrScale = 1.0,
    bool reducedMotion = false,
  })  : _textScale = _clampText(textScale),
        _qrScale = _clampQr(qrScale),
        _reducedMotion = reducedMotion;

  /// Paliers proposés à l'utilisateur (futur écran de réglages).
  static const textScaleSteps = [0.85, 1.0, 1.25, 1.5, 2.0];

  double _textScale;
  double _qrScale;
  bool _reducedMotion;

  static double _clampText(double v) => v.clamp(0.8, 2.0).toDouble();
  static double _clampQr(double v) => v.clamp(0.6, 1.6).toDouble();

  /// Multiplicateur appliqué à TOUS les textes de l'app.
  double get textScale => _textScale;
  set textScale(double v) {
    _textScale = _clampText(v);
    notifyListeners();
  }

  /// Multiplicateur de la taille des QR codes.
  double get qrScale => _qrScale;
  set qrScale(double v) {
    _qrScale = _clampQr(v);
    notifyListeners();
  }

  /// Désactive toutes les animations du design system.
  bool get reducedMotion => _reducedMotion;
  set reducedMotion(bool v) {
    _reducedMotion = v;
    notifyListeners();
  }

  void increaseText() => _step(1);
  void decreaseText() => _step(-1);

  void _step(int dir) {
    var i = textScaleSteps.indexWhere((s) => s >= _textScale - 0.001);
    if (i < 0) i = textScaleSteps.length - 1;
    textScale = textScaleSteps[(i + dir).clamp(0, textScaleSteps.length - 1)];
  }
}

/// Rend [UiSettings] accessible via UiSettingsScope.of(context).
class UiSettingsScope extends InheritedNotifier<UiSettings> {
  const UiSettingsScope({super.key, required UiSettings settings, required super.child})
      : super(notifier: settings);

  static UiSettings of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<UiSettingsScope>();
    assert(scope != null, 'Aucun AppUiScope au-dessus de ce widget.');
    return scope!.notifier!;
  }
}

/// À placer dans MaterialApp.builder : applique les réglages à toute l'app
/// (taille du texte via MediaQuery, animations désactivées).
class AppUiScope extends StatelessWidget {
  const AppUiScope({super.key, required this.settings, required this.child});

  final UiSettings settings;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return UiSettingsScope(
      settings: settings,
      child: Builder(builder: (context) {
        final s = UiSettingsScope.of(context);
        final mq = MediaQuery.of(context);
        return MediaQuery(
          data: mq.copyWith(
            textScaler: TextScaler.linear(s.textScale),
            disableAnimations: s.reducedMotion || mq.disableAnimations,
          ),
          child: child,
        );
      }),
    );
  }
}