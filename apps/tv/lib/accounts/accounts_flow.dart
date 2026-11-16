import 'package:epikodi_client/epikodi_client.dart' show ApiException;
import 'package:flutter/material.dart';

import '../../api/api.dart';
import '../library/library_screen.dart';
import 'accounts_screen.dart';
import 'create_account_screen.dart';
import 'pin_dialog.dart';

/// Relie les écrans de choix de compte :
///
///   « Qui regarde ? » ──┬─ compte ──(PIN si requis)──┐
///                       ├─ Invité ───────────────────┼─> POST /api/session/profile ─> Médiathèque
///                       └─ Créer ─> CreateAccountScreen (POST /api/profiles) ─────────┘
///
/// C'est le seul endroit qui connaît à la fois l'API et la navigation :
/// les écrans, eux, restent de simples vues.
class AccountsFlow extends StatefulWidget {
  const AccountsFlow({
    super.key,
    required this.api,
    required this.data,
    required this.onBackToPairing,
  });

  final Api api;

  /// Réponse du serveur : { phase, profiles: [...], guest: {...}, can_create }.
  final Map<String, dynamic> data;

  /// Retour à l'écran d'appairage.
  final VoidCallback onBackToPairing;

  @override
  State<AccountsFlow> createState() => _AccountsFlowState();
}

class _AccountsFlowState extends State<AccountsFlow> {
  /// Copie locale : on y ajoute les comptes créés pendant la session.
  late Map<String, dynamic> _data = Map<String, dynamic>.of(widget.data);

  /// Évite les doubles appuis pendant un appel réseau.
  bool _busy = false;

  @override
  void didUpdateWidget(AccountsFlow old) {
    super.didUpdateWidget(old);
    if (old.data != widget.data) {
      _data = Map<String, dynamic>.of(widget.data);
    }
  }

  // ------------------------------------------------------------- navigation

  Future<void> _openLibrary(Map<String, dynamic> profile) {
    return Navigator.of(context).push(MaterialPageRoute<void>(
      builder: (ctx) => LibraryScreen(
        api: widget.api,
        profile: profile,
        // Retour à « Qui regarde ? ».
        onChangeProfile: () => Navigator.of(ctx).pop(),
        onBackToPairing: widget.onBackToPairing,
      ),
    ));
  }

  void _showError(String message) {
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(message)));
  }

  // ---------------------------------------------------------------- actions

  Future<void> _pick(Map<String, dynamic> p) async {
    if (_busy) return;
    final id = p['id']; // entier, comme dans la fixture

    if (p['requires_pin'] == true) {
      Map<String, dynamic>? active;
      await showDialog<void>(
        context: context,
        builder: (_) => PinDialog(
          profileName: '${p['name']}',
          onSubmit: (pin) async {
            try {
              active = await widget.api.selectProfile(profileId: id, pin: pin);
              return null; // PIN bon : la popup se ferme.
            } on ApiException catch (e) {
              return e.message; // Affiché dans la popup, saisie effacée.
            }
          },
        ),
      );
      final result = active;
      if (result != null && mounted) await _openLibrary(result);
      return;
    }

    await _run(() => widget.api.selectProfile(profileId: id));
  }

  Future<void> _guest() => _run(() => widget.api.selectProfile(guest: true));

  Future<void> _create() async {
    if (_busy) return;
    final created = await Navigator.of(context).push<Map<String, dynamic>>(
      MaterialPageRoute(
        builder: (_) => CreateAccountScreen(api: widget.api),
      ),
    );
    if (created == null || !mounted) return; // Annulé.

    // Le compte apparaît dans « Qui regarde ? » au retour.
    setState(() {
      _data = {
        ..._data,
        'profiles': [...(_data['profiles'] as List? ?? const []), created],
      };
    });
    // Le serveur l'a déjà rendu actif : on entre directement.
    await _openLibrary(created);
  }

  /// Appel réseau + ouverture de la médiathèque, avec gestion d'erreur.
  Future<void> _run(Future<Map<String, dynamic>> Function() call) async {
    if (_busy) return;
    _busy = true;
    try {
      final profile = await call();
      if (mounted) await _openLibrary(profile);
    } on ApiException catch (e) {
      if (mounted) _showError(e.message);
    } finally {
      _busy = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    return AccountsScreen(
      data: _data,
      onBackToPairing: widget.onBackToPairing,
      onProfile: _pick,
      onGuest: _guest,
      onCreate: _create,
    );
  }
}
