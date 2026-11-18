import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

import 'package:epikodi_client/epikodi_client.dart' show ApiException;

import '../api/api.dart';
import '../l10n/app_localizations.dart';
import '../profile/profile_screen.dart';
import '../widgets/profile_menu.dart';

/// Médiathèque (version simple) : appelle GET /api/media et affiche les
/// médias du profil actif en rangées d'affiches (films, séries, musique,
/// podcasts). Le serveur filtre déjà selon l'âge du profil.
/// Ouvrir un média (saisons, lecture) n'est pas encore branché.
///
/// En haut à gauche : le logo. En haut à droite : l'icône du profil, qui
/// ouvre un menu déroulant (« Profil », « Changer de profil » ou retour à
/// l'appairage).
class LibraryScreen extends StatefulWidget {
  const LibraryScreen({
    super.key,
    required this.api,
    required this.profile,
    required this.onChangeProfile,
    required this.onBackToPairing,
  });

  final Api api;

  /// Compte actif (réponse de POST /api/session/profile).
  final Map<String, dynamic> profile;

  /// Retour à l'écran « Qui regarde ? » (depuis le menu du profil).
  final VoidCallback onChangeProfile;

  /// Retour à l'écran d'appairage (depuis le menu du profil).
  final VoidCallback onBackToPairing;

  @override
  State<LibraryScreen> createState() => _LibraryScreenState();
}

class _LibraryScreenState extends State<LibraryScreen> {
  /// Ordre et titres des rangées, selon le champ `kind` du média.
  Map<String, String> _sections(AppLocalizations l10n) => {
        'movie': l10n.sectionMovies,
        'series': l10n.sectionSeries,
        'artist': l10n.sectionMusic,
        'podcast': l10n.sectionPodcasts,
      };

  late Future<List<Map<String, dynamic>>> _future = widget.api.fetchMedia();

  // Copie locale du profil : mise à jour après une modification.
  late Map<String, dynamic> _profile = widget.profile;

  // Accolades obligatoires : `setState(() => _future = ...)` renverrait un
  // Future, ce que setState refuse (assertion en debug, bouton sans effet).
  void _reload() => setState(() {
        _future = widget.api.fetchMedia();
      });

  Future<void> _openProfile() async {
    final updated = await Navigator.of(context).push<Map<String, dynamic>>(
      MaterialPageRoute(
        builder: (_) => ProfileScreen(api: widget.api, profile: _profile),
      ),
    );
    if (updated == null || !mounted) return;
    setState(() {
      _profile = {..._profile, ...updated};
      // L'âge ou la restriction ont pu changer : le serveur re-filtre les médias.
      _future = widget.api.fetchMedia();
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final name = '${_profile['name'] ?? l10n.guestName}';

    return Scaffold(
      body: BrandBackground(
        child: TvSafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.lg,
              vertical: AppSpacing.md,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const GloubiLogo(size: 56),
                    ProfileMenuButton(
                      name: name,
                      onProfile: _openProfile,
                      onSwitchProfile: widget.onChangeProfile,
                      onPairing: widget.onBackToPairing,
                    ),
                  ],
                ),
                const SizedBox(height: AppSpacing.md),
                Expanded(
                  child: FutureBuilder<List<Map<String, dynamic>>>(
                    future: _future,
                    builder: (context, snap) {
                      if (snap.connectionState != ConnectionState.done) {
                        return const Center(child: CircularProgressIndicator());
                      }
                      if (snap.hasError) {
                        final message = snap.error is ApiException
                            ? (snap.error as ApiException).message
                            : l10n.libraryLoadError;
                        return _Message(
                          worried: true,
                          text: message,
                          actionText: l10n.retry,
                          onAction: _reload,
                        );
                      }
                      final items = snap.data ?? const [];
                      final rows = <Widget>[];
                      var first = true;
                      for (final entry in _sections(l10n).entries) {
                        final list =
                            items.where((m) => m['kind'] == entry.key).toList();
                        if (list.isEmpty) continue;
                        rows.add(_MediaRow(
                          title: entry.value,
                          items: list,
                          baseUrl: widget.api.baseUrl,
                          autofocusFirst: first,
                        ));
                        first = false;
                      }
                      if (rows.isEmpty) {
                        return _Message(
                          icon: Icons.video_library_outlined,
                          text: l10n.noMediaForProfile,
                        );
                      }
                      return SingleChildScrollView(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: rows,
                        ),
                      );
                    },
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Une rangée d'affiches qui défile horizontalement.
class _MediaRow extends StatelessWidget {
  const _MediaRow({
    required this.title,
    required this.items,
    required this.baseUrl,
    required this.autofocusFirst,
  });

  final String title;
  final List<Map<String, dynamic>> items;
  final String baseUrl;
  final bool autofocusFirst;

  @override
  Widget build(BuildContext context) {
    final scaler = MediaQuery.textScalerOf(context);
    final cardWidth = scaler.scale(160);
    final posterHeight = cardWidth * 1.5;
    final labelHeight = scaler
        .scale((AppText.body.fontSize ?? 20) * (AppText.body.height ?? 1.4));
    // Affiche + marges de la carte + libellé + marge pour le halo de focus.
    final rowHeight = posterHeight +
        2 * AppSpacing.sm +
        AppSpacing.sm +
        labelHeight +
        2 * AppSpacing.md;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(title, style: AppText.title),
        SizedBox(
          height: rowHeight,
          child: ListView.separated(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(vertical: AppSpacing.md),
            itemCount: items.length,
            separatorBuilder: (_, __) => const SizedBox(width: AppSpacing.md),
            itemBuilder: (_, i) => _MediaCard(
              item: items[i],
              baseUrl: baseUrl,
              width: cardWidth,
              autofocus: autofocusFirst && i == 0,
            ),
          ),
        ),
        const SizedBox(height: AppSpacing.sm),
      ],
    );
  }
}

class _MediaCard extends StatelessWidget {
  const _MediaCard({
    required this.item,
    required this.baseUrl,
    required this.width,
    required this.autofocus,
  });

  final Map<String, dynamic> item;
  final String baseUrl;
  final double width;
  final bool autofocus;

  @override
  Widget build(BuildContext context) {
    final title = '${item['title']}';
    final posterUrl = item['poster_url'] as String?;
    final placeholder = Container(
      width: width,
      height: width * 1.5,
      color: AppColors.surfaceHigh,
      alignment: Alignment.center,
      child: Icon(Icons.movie_outlined,
          size: width * 0.4, color: AppColors.textMuted),
    );

    return FocusableCard(
      autofocus: autofocus,
      onTap: () {
        // À brancher : ouvrir la fiche, les saisons (parent_id) ou lancer la lecture.
      },
      borderRadius: AppRadius.lg,
      padding: const EdgeInsets.all(AppSpacing.sm),
      semanticLabel: title,
      // Si la place manque, la carte se réduit au lieu de déborder.
      child: FittedBox(
        fit: BoxFit.scaleDown,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(AppRadius.lg / 2),
              child: posterUrl == null
                  ? placeholder
                  : Image.network(
                      '$baseUrl$posterUrl',
                      width: width,
                      height: width * 1.5,
                      fit: BoxFit.cover,
                      errorBuilder: (_, __, ___) => placeholder,
                    ),
            ),
            const SizedBox(height: AppSpacing.sm),
            SizedBox(
              width: width,
              child: Text(
                title,
                style: AppText.body,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Message centré (chargement impossible, médiathèque vide), avec un bouton
/// optionnel.
class _Message extends StatelessWidget {
  const _Message({
    this.icon,
    this.worried = false,
    required this.text,
    this.actionText,
    this.onAction,
  });

  /// Icône au-dessus du texte (si [worried] est faux).
  final IconData? icon;

  /// Erreur : Gloubi inquiet à la place de l'icône.
  final bool worried;
  final String text;
  final String? actionText;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    final scaler = MediaQuery.textScalerOf(context);
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (worried)
            Gloubi(
              animation: GloubiAnimation.worried,
              size: scaler.scale(160),
            )
          else if (icon != null)
            Icon(icon, size: scaler.scale(56), color: AppColors.textMuted),
          const SizedBox(height: AppSpacing.sm),
          Text(text, style: AppText.title, textAlign: TextAlign.center),
          if (actionText != null) ...[
            const SizedBox(height: AppSpacing.md),
            FocusableCard(
              autofocus: true,
              borderRadius: 999,
              onTap: onAction ?? () {},
              semanticLabel: actionText!,
              padding: const EdgeInsets.symmetric(
                horizontal: AppSpacing.lg,
                vertical: AppSpacing.sm,
              ),
              child: Text(actionText!,
                  style: AppText.body.copyWith(fontWeight: FontWeight.w600)),
            ),
          ],
        ],
      ),
    );
  }
}
