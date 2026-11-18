import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import '../widgets/pill_choice.dart';
import 'playback_engine.dart';
import 'playback_state.dart';
import 'subtitle_style.dart';

/// Ouvre le panneau « Audio et sous-titres » sur le côté droit de l'écran.
/// Les choix s'appliquent tout de suite ; Retour ferme le panneau.
Future<void> showTrackPanel(
  BuildContext context, {
  required PlaybackEngine engine,
  required ValueNotifier<SubtitleStyle> subtitleStyle,
}) {
  final l10n = AppLocalizations.of(context)!;
  return showGeneralDialog<void>(
    context: context,
    barrierDismissible: true,
    barrierLabel: l10n.back,
    barrierColor: const Color(0x99000000),
    // Fondu court, supprimé si les animations sont désactivées.
    transitionDuration: MediaQuery.disableAnimationsOf(context)
        ? Duration.zero
        : AppDurations.fast,
    pageBuilder: (context, _, __) => Shortcuts(
      // Touches « Retour » de télécommande, en plus d'Échap (déjà géré).
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.goBack): DismissIntent(),
        SingleActivator(LogicalKeyboardKey.browserBack): DismissIntent(),
        SingleActivator(LogicalKeyboardKey.backspace): DismissIntent(),
      },
      child: Align(
        alignment: Alignment.centerRight,
        child: TrackPanel(engine: engine, subtitleStyle: subtitleStyle),
      ),
    ),
  );
}

/// Contenu du panneau : pistes audio, sous-titres, style des sous-titres.
class TrackPanel extends StatelessWidget {
  const TrackPanel({
    super.key,
    required this.engine,
    required this.subtitleStyle,
  });

  final PlaybackEngine engine;
  final ValueNotifier<SubtitleStyle> subtitleStyle;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final width = MediaQuery.sizeOf(context).width;

    return Material(
      color: AppColors.surface,
      child: SizedBox(
        width: (width * 0.4).clamp(420.0, 720.0),
        height: double.infinity,
        child: SafeArea(
          child: FocusTraversalGroup(
            child: ListenableBuilder(
              listenable: Listenable.merge([engine.state, subtitleStyle]),
              builder: (context, _) {
                final state = engine.state.value;
                final style = subtitleStyle.value;
                void setStyle(SubtitleStyle s) => subtitleStyle.value = s;

                return ListView(
                  padding: const EdgeInsets.all(AppSpacing.lg),
                  children: [
                    Semantics(
                      header: true,
                      child: Text(l10n.playerTracks, style: AppText.headline),
                    ),
                    _Section(
                      title: l10n.playerAudio,
                      vertical: true,
                      children: [
                        if (state.audioTracks.isEmpty)
                          Text(l10n.playerNoTrack, style: AppText.label),
                        for (final (i, t) in state.audioTracks.indexed)
                          PillChoice(
                            text: trackLabel(t, i, l10n),
                            selected: t == state.audioTrack,
                            semanticPrefix: l10n.playerAudio,
                            compact: true,
                            // Focus d'ouverture : premier choix du panneau.
                            autofocus: i == 0,
                            onTap: () => engine.setAudioTrack(t),
                          ),
                      ],
                    ),
                    _Section(
                      title: l10n.playerSubtitles,
                      vertical: true,
                      children: [
                        PillChoice(
                          text: l10n.playerSubtitlesOff,
                          selected: state.subtitleTrack == null,
                          semanticPrefix: l10n.playerSubtitles,
                          compact: true,
                          autofocus: state.audioTracks.isEmpty,
                          onTap: () => engine.setSubtitleTrack(null),
                        ),
                        for (final (i, t) in state.subtitleTracks.indexed)
                          PillChoice(
                            text: trackLabel(t, i, l10n),
                            selected: t == state.subtitleTrack,
                            semanticPrefix: l10n.playerSubtitles,
                            compact: true,
                            onTap: () => engine.setSubtitleTrack(t),
                          ),
                      ],
                    ),
                    _Section(
                      title: l10n.subtitleSize,
                      children: [
                        for (final v in SubtitleSize.values)
                          PillChoice(
                            text: _sizeLabel(v, l10n),
                            selected: style.size == v,
                            semanticPrefix: l10n.subtitleSize,
                            compact: true,
                            onTap: () => setStyle(style.copyWith(size: v)),
                          ),
                      ],
                    ),
                    _Section(
                      title: l10n.subtitleColor,
                      children: [
                        for (final v in SubtitleColor.values)
                          PillChoice(
                            text: _colorLabel(v, l10n),
                            selected: style.color == v,
                            semanticPrefix: l10n.subtitleColor,
                            compact: true,
                            onTap: () => setStyle(style.copyWith(color: v)),
                          ),
                      ],
                    ),
                    _Section(
                      title: l10n.subtitleBackground,
                      children: [
                        for (final v in SubtitleBackground.values)
                          PillChoice(
                            text: _backgroundLabel(v, l10n),
                            selected: style.background == v,
                            semanticPrefix: l10n.subtitleBackground,
                            compact: true,
                            onTap: () =>
                                setStyle(style.copyWith(background: v)),
                          ),
                      ],
                    ),
                    const SizedBox(height: AppSpacing.md),
                    // Aperçu sur fond vidéo simulé (gris moyen) : on juge la
                    // lisibilité avant de refermer le panneau.
                    ExcludeSemantics(
                      child: Container(
                        padding: const EdgeInsets.all(AppSpacing.md),
                        color: const Color(0xFF5A6070),
                        alignment: Alignment.center,
                        child: Text(
                          l10n.subtitlePreview,
                          style: style.toTextStyle(),
                          textAlign: TextAlign.center,
                        ),
                      ),
                    ),
                  ],
                );
              },
            ),
          ),
        ),
      ),
    );
  }
}

/// Libellé d'une piste : titre et langue du fichier, sinon « Piste N ».
/// L'audiodescription est écrite en toutes lettres.
String trackLabel(MediaTrack t, int index, AppLocalizations l10n) {
  final parts = [
    if (t.title != null && t.title!.trim().isNotEmpty) t.title!.trim(),
    if (t.language != null && t.language!.isNotEmpty) t.language!.toUpperCase(),
  ];
  var label =
      parts.isEmpty ? l10n.playerTrackNumber(index + 1) : parts.join(' · ');
  if (t.isAudioDescription &&
      !label
          .toLowerCase()
          .contains(l10n.playerAudioDescription.toLowerCase())) {
    label = '$label · ${l10n.playerAudioDescription}';
  }
  return label;
}

String _sizeLabel(SubtitleSize v, AppLocalizations l10n) => switch (v) {
      SubtitleSize.small => l10n.subtitleSizeSmall,
      SubtitleSize.normal => l10n.subtitleSizeNormal,
      SubtitleSize.large => l10n.subtitleSizeLarge,
      SubtitleSize.extraLarge => l10n.subtitleSizeExtraLarge,
    };

String _colorLabel(SubtitleColor v, AppLocalizations l10n) => switch (v) {
      SubtitleColor.white => l10n.subtitleColorWhite,
      SubtitleColor.yellow => l10n.subtitleColorYellow,
      SubtitleColor.cyan => l10n.subtitleColorCyan,
    };

String _backgroundLabel(SubtitleBackground v, AppLocalizations l10n) =>
    switch (v) {
      SubtitleBackground.none => l10n.subtitleBackgroundNone,
      SubtitleBackground.translucent => l10n.subtitleBackgroundTranslucent,
      SubtitleBackground.opaque => l10n.subtitleBackgroundOpaque,
    };

/// Titre de section suivi de ses choix : une ligne par choix ([vertical],
/// pour les pistes aux libellés longs) ou à la suite, avec retour à la ligne.
class _Section extends StatelessWidget {
  const _Section({
    required this.title,
    required this.children,
    this.vertical = false,
  });

  final String title;
  final List<Widget> children;
  final bool vertical;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.only(top: AppSpacing.md),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Semantics(
              header: true,
              child: Text(title, style: AppText.title),
            ),
            const SizedBox(height: AppSpacing.xs),
            if (vertical)
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final c in children)
                    Padding(
                      padding: const EdgeInsets.only(bottom: AppSpacing.xs),
                      child: c,
                    ),
                ],
              )
            else
              Wrap(
                spacing: AppSpacing.sm,
                runSpacing: AppSpacing.sm,
                children: children,
              ),
          ],
        ),
      );
}
