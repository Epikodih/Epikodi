import 'package:epikodi_client/epikodi_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import '../locale.dart';
import '../pairing/tv_events.dart';

/// Champ texte qui a le focus sur la TV, tel qu'il est annoncé au téléphone
/// (qui ouvre alors son clavier : la TV, elle, n'en a pas).
class TextFieldInfo {
  const TextFieldInfo({
    required this.id,
    required this.label,
    required this.value,
    required this.obscure,
    required this.number,
    this.maxLength,
  });

  /// Identifie le champ : change quand le focus passe à un autre champ.
  final int id;
  final String label;
  final String value;
  final bool obscure;

  /// Vrai : clavier numérique sur le téléphone.
  final bool number;
  final int? maxLength;

  Map<String, Object?> toJson() => {
        'active': true,
        'id': id,
        'label': label,
        'value': value,
        'obscure': obscure,
        'keyboard': number ? 'number' : 'text',
        'max_length': maxLength,
      };
}

/// Retour à l'écran d'appairage, demandé par la télécommande (raccourci
/// `pairing`, quand le téléphone se désappaire). Rempli par l'écran
/// d'appairage quand il passe la main aux comptes ; `null` sur l'appairage.
VoidCallback? remoteBackToPairing;

/// Champ texte qui a le focus, ou `null`.
EditableTextState? focusedEditable() {
  final context = FocusManager.instance.primaryFocus?.context;
  if (context == null || !context.mounted) return null;
  if (context is StatefulElement && context.state is EditableTextState) {
    return context.state as EditableTextState;
  }
  final state = context.findAncestorStateOfType<EditableTextState>();
  // Seulement si le focus est bien celui du champ (pas un bouton dedans).
  return state != null &&
          state.widget.focusNode == FocusManager.instance.primaryFocus
      ? state
      : null;
}

/// Décrit le champ texte qui a le focus (libellé, valeur…), ou `null`.
TextFieldInfo? focusedTextField() {
  final state = focusedEditable();
  if (state == null) return null;
  final field = state.context.findAncestorWidgetOfExactType<TextField>();
  final decoration = field?.decoration;
  final w = state.widget;
  return TextFieldInfo(
    id: identityHashCode(w.focusNode),
    label: decoration?.labelText ?? decoration?.hintText ?? '',
    value: w.controller.text,
    obscure: w.obscureText,
    number: w.keyboardType == TextInputType.number ||
        w.keyboardType == TextInputType.phone,
    maxLength: field?.maxLength,
  );
}

/// Touches de la télécommande -> mêmes effets que le clavier de la TV.
///
/// Chaque touche est d'abord proposée au widget qui a le focus, comme une
/// vraie touche (`FocusableCard` réagit à Entrée, les écrans à Échap…).
/// Si personne ne la prend :
///  - flèches : focus dans cette direction, sinon focus suivant / précédent
///    (comme Tab / Maj+Tab) ;
///  - OK : active l'élément qui a le focus ;
///  - Retour : écran précédent ;
///  - Accueil : premier écran.
///
/// Dans un champ texte, les flèches font toujours sortir du champ (le champ,
/// lui, les garderait pour son curseur) : on tape sur le téléphone, pas avec
/// les flèches. OK sur un champ texte rouvre le clavier du téléphone.
class RemoteKeyHandler {
  RemoteKeyHandler(this.navigator, {this.onTextFieldOk});

  final GlobalKey<NavigatorState> navigator;

  /// OK sur un champ texte : redemander le clavier au téléphone.
  final VoidCallback? onTextFieldOk;

  static const _arrows = {
    'up': (
      LogicalKeyboardKey.arrowUp,
      PhysicalKeyboardKey.arrowUp,
      TraversalDirection.up
    ),
    'down': (
      LogicalKeyboardKey.arrowDown,
      PhysicalKeyboardKey.arrowDown,
      TraversalDirection.down
    ),
    'left': (
      LogicalKeyboardKey.arrowLeft,
      PhysicalKeyboardKey.arrowLeft,
      TraversalDirection.left
    ),
    'right': (
      LogicalKeyboardKey.arrowRight,
      PhysicalKeyboardKey.arrowRight,
      TraversalDirection.right
    ),
  };

  /// Événement du serveur : `{"key": "up"}`, `{"text": "Léa", "done": false}`
  /// ou `{"shortcut": "language", "language": "en"}`.
  void handle(Map<String, dynamic> event) {
    final key = event['key'];
    if (key is String) press(key);
    final text = event['text'];
    if (text is String) typeText(text, done: event['done'] == true);
    if (event['shortcut'] == 'language') setLanguage(event['language']);
    if (event['shortcut'] == 'pairing') remoteBackToPairing?.call();
  }

  /// Raccourci « langue » : change la langue de la TV (fr ou en).
  void setLanguage(Object? code) {
    if (code is! String) return;
    final supported = AppLocalizations.supportedLocales
        .any((l) => l.languageCode == code);
    if (supported) appLocale.value = Locale(code);
  }

  /// Applique une touche : up, down, left, right, ok, back, home.
  void press(String key) {
    final inText = focusedEditable() != null;
    final arrow = _arrows[key];
    if (arrow != null) {
      final (logical, physical, direction) = arrow;
      if (!inText && _dispatch(logical, physical)) return;
      _move(direction);
      return;
    }
    switch (key) {
      case 'ok':
        if (inText) {
          onTextFieldOk?.call();
          return;
        }
        if (_dispatch(LogicalKeyboardKey.enter, PhysicalKeyboardKey.enter)) {
          return;
        }
        final context = FocusManager.instance.primaryFocus?.context;
        if (context != null) {
          Actions.maybeInvoke(context, const ActivateIntent());
        }
      case 'back':
        if (_dispatch(LogicalKeyboardKey.escape, PhysicalKeyboardKey.escape)) {
          return;
        }
        navigator.currentState?.maybePop();
      case 'home':
        navigator.currentState?.popUntil((route) => route.isFirst);
    }
  }

  /// Texte tapé sur le téléphone : remplace celui du champ qui a le focus
  /// (avec ses limites : longueur max, filtres). [done] : le texte est
  /// validé, le focus passe à l'élément d'en dessous (sinon à droite, sinon
  /// au suivant). Pas de « Entrée » : sur certains écrans, il renverrait le
  /// focus tout en haut de la page.
  void typeText(String text, {bool done = false}) {
    final state = focusedEditable();
    if (state == null) return;
    state.userUpdateTextEditingValue(
      TextEditingValue(
        text: text,
        selection: TextSelection.collapsed(offset: text.length),
      ),
      SelectionChangedCause.keyboard,
    );
    if (done) _leaveTextField(state.widget.focusNode);
  }

  void _leaveTextField(FocusNode field) {
    if (field.focusInDirection(TraversalDirection.down)) return;
    if (field.focusInDirection(TraversalDirection.right)) return;
    field.nextFocus();
  }

  /// Envoie une fausse touche au focus, puis à ses parents, comme le
  /// clavier. Vrai si un widget l'a prise.
  bool _dispatch(LogicalKeyboardKey logical, PhysicalKeyboardKey physical) {
    final event = KeyDownEvent(
      physicalKey: physical,
      logicalKey: logical,
      timeStamp: Duration.zero,
    );
    for (var node = FocusManager.instance.primaryFocus;
        node != null;
        node = node.parent) {
      final result = node.onKeyEvent?.call(node, event);
      if (result == KeyEventResult.handled) return true;
      if (result == KeyEventResult.skipRemainingHandlers) return false;
    }
    return false;
  }

  /// Déplace le focus. Rien de focalisé : premier élément. Rien dans cette
  /// direction : élément suivant / précédent (ordre de tabulation).
  void _move(TraversalDirection direction) {
    final node = FocusManager.instance.primaryFocus;
    if (node == null || node is FocusScopeNode) {
      (node ?? FocusManager.instance.rootScope).nextFocus();
      return;
    }
    if (node.focusInDirection(direction)) return;
    if (direction == TraversalDirection.down ||
        direction == TraversalDirection.right) {
      node.nextFocus();
    } else {
      node.previousFocus();
    }
  }
}

/// Suit le focus : appelle [onChanged] quand un champ texte le prend ou le
/// perd (`null`). La TV prévient ainsi le téléphone d'ouvrir son clavier.
class TextFieldWatcher {
  TextFieldWatcher(this.onChanged) {
    FocusManager.instance.addListener(_check);
  }

  final void Function(TextFieldInfo? field) onChanged;
  int? _current;

  void _check() {
    final field = focusedTextField();
    if (field?.id == _current) return;
    _current = field?.id;
    onChanged(field);
  }

  /// Annonce de nouveau le champ actuel (OK sur un champ texte).
  void resend() => onChanged(focusedTextField());

  void dispose() => FocusManager.instance.removeListener(_check);
}

/// Pendant toute la vie de l'app (routes provisoires du faux serveur
/// `test_api.py`, pas encore dans les fixtures) :
///  - écoute la télécommande (`GET /api/tv/remote`) : touches et texte ;
///  - annonce le champ texte qui a le focus (`POST /api/tv/text-input`),
///    pour que le téléphone ouvre son clavier.
class RemoteKeyListener extends StatefulWidget {
  const RemoteKeyListener({
    super.key,
    required this.baseUrl,
    required this.navigator,
    required this.child,
  });

  final String baseUrl;
  final GlobalKey<NavigatorState> navigator;
  final Widget child;

  @override
  State<RemoteKeyListener> createState() => _RemoteKeyListenerState();
}

class _RemoteKeyListenerState extends State<RemoteKeyListener> {
  late final TvEvents _events;
  late final TextFieldWatcher _watcher;
  late final EpikodiClient _client = EpikodiClient(baseUrl: widget.baseUrl);

  @override
  void initState() {
    super.initState();
    _watcher = TextFieldWatcher(_announce);
    final handler =
        RemoteKeyHandler(widget.navigator, onTextFieldOk: _watcher.resend);
    _events = TvEvents(
      baseUrl: widget.baseUrl,
      sessionId: '',
      path: '/api/tv/remote',
      onEvent: handler.handle,
    )..start();
  }

  Future<void> _announce(TextFieldInfo? field) async {
    try {
      await _client.http.post('/api/tv/text-input',
          body: field?.toJson() ?? const {'active': false});
    } on ApiException {
      // Pas de serveur ou route absente : la TV reste utilisable au clavier.
    }
  }

  @override
  void dispose() {
    _watcher.dispose();
    _events.close();
    _client.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
