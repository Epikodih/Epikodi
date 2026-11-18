import 'dart:async';

import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import 'playback_engine.dart';
import 'playback_state.dart';
import 'subtitle_style.dart';
import 'time_format.dart';
import 'track_panel.dart';

/// Écran de lecture plein écran, piloté à la télécommande.
///
/// Touches (contrôles masqués) :
///   Entrée / OK / Espace   lecture ↔ pause
///   ← / →                  recule de 10 s / avance de 30 s
///   ↑ / ↓                  affiche les contrôles
///   Retour / Échap         quitte la lecture
///
/// Contrôles affichés : les flèches déplacent le focus entre les boutons,
/// Entrée active le bouton. Ils se masquent après [hideDelay] sans touche,
/// uniquement pendant la lecture (en pause, ils restent visibles).
class PlayerScreen extends StatefulWidget {
  const PlayerScreen({
    super.key,
    required this.engine,
    required this.source,
    this.hideDelay = const Duration(seconds: 5),
    this.subtitleStyle = const SubtitleStyle(),
    this.onSubtitleStyleChanged,
  });

  /// Moteur de lecture. L'écran l'ouvre, puis le libère en se fermant.
  final PlaybackEngine engine;
  final PlaybackSource source;
  final Duration hideDelay;

  /// Apparence des sous-titres au lancement (réglée par le profil).
  final SubtitleStyle subtitleStyle;

  /// Appelé quand l'utilisateur change l'apparence dans le panneau, pour la
  /// garder d'une lecture à l'autre.
  final ValueChanged<SubtitleStyle>? onSubtitleStyleChanged;

  static const rewindStep = Duration(seconds: 10);
  static const forwardStep = Duration(seconds: 30);

  @override
  State<PlayerScreen> createState() => _PlayerScreenState();
}

class _PlayerScreenState extends State<PlayerScreen> {
  // skipTraversal : l'écran reçoit les touches qui remontent des boutons,
  // mais les flèches ne doivent jamais lui donner le focus.
  final _rootFocus = FocusNode(debugLabel: 'player', skipTraversal: true);
  final _playFocus = FocusNode(debugLabel: 'player.play');
  late final _subtitleStyle = ValueNotifier(widget.subtitleStyle);
  Timer? _hideTimer;
  bool _panelOpen = false;
  bool _controlsVisible = true;

  PlaybackEngine get _engine => widget.engine;

  static final _selectKeys = {
    LogicalKeyboardKey.select,
    LogicalKeyboardKey.enter,
    LogicalKeyboardKey.numpadEnter,
    LogicalKeyboardKey.gameButtonA,
    LogicalKeyboardKey.space,
  };

  static final _backKeys = {
    LogicalKeyboardKey.escape,
    LogicalKeyboardKey.goBack,
    LogicalKeyboardKey.browserBack,
    LogicalKeyboardKey.backspace,
    LogicalKeyboardKey.gameButtonB,
  };

  @override
  void initState() {
    super.initState();
    _engine.state.addListener(_onEngineState);
    _subtitleStyle.addListener(
        () => widget.onSubtitleStyleChanged?.call(_subtitleStyle.value));
    _engine.open(widget.source);
    _scheduleHide();
  }

  @override
  void dispose() {
    _hideTimer?.cancel();
    _engine.state.removeListener(_onEngineState);
    _rootFocus.dispose();
    _playFocus.dispose();
    _subtitleStyle.dispose();
    _engine.dispose();
    super.dispose();
  }

  // Vrai si la lecture avance (ni pause, ni erreur, ni fin).
  bool _wasRunning = false;

  // Réagit seulement aux changements lecture ↔ arrêt (pas à chaque position) :
  // en pause, en erreur ou à la fin, les contrôles restent visibles.
  void _onEngineState() {
    final s = _engine.state.value;
    final running = s.playing && s.error == null && !s.completed;
    if (running == _wasRunning) return;
    _wasRunning = running;
    if (running) {
      _scheduleHide();
    } else {
      // Le délai relancé ici ne masque rien : il vérifie que la lecture avance.
      _showControls();
    }
  }

  void _scheduleHide() {
    _hideTimer?.cancel();
    _hideTimer = Timer(widget.hideDelay, () {
      final s = _engine.state.value;
      if (!mounted || _panelOpen || !s.playing || s.error != null) return;
      setState(() => _controlsVisible = false);
      // Le focus revient à l'écran : les flèches pilotent alors la lecture.
      _rootFocus.requestFocus();
    });
  }

  void _showControls() {
    if (!mounted) return;
    if (!_controlsVisible) {
      setState(() => _controlsVisible = true);
      WidgetsBinding.instance
          .addPostFrameCallback((_) => _playFocus.requestFocus());
    }
    _scheduleHide();
  }

  /// Panneau « Audio et sous-titres » : les contrôles restent affichés
  /// pendant qu'il est ouvert, puis le focus revient sur Lecture / Pause.
  Future<void> _openTrackPanel() async {
    _panelOpen = true;
    _hideTimer?.cancel();
    await showTrackPanel(context,
        engine: _engine, subtitleStyle: _subtitleStyle);
    _panelOpen = false;
    if (!mounted) return;
    _playFocus.requestFocus();
    _scheduleHide();
  }

  Future<void> _seekBy(Duration delta) =>
      _engine.seek(_engine.state.value.position + delta);

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final key = event.logicalKey;

    if (_backKeys.contains(key)) {
      Navigator.of(context).maybePop();
      return KeyEventResult.handled;
    }

    // Touches multimédia : actives que les contrôles soient visibles ou non.
    if (key == LogicalKeyboardKey.mediaPlayPause) {
      _engine.togglePlay();
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.mediaPlay) {
      _engine.play();
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.mediaPause) {
      _engine.pause();
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.mediaRewind) {
      _seekBy(-PlayerScreen.rewindStep);
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.mediaFastForward) {
      _seekBy(PlayerScreen.forwardStep);
      return KeyEventResult.handled;
    }

    // Contrôles visibles : les boutons gèrent Entrée, les flèches déplacent
    // le focus. On relance seulement le délai de masquage.
    if (_controlsVisible) {
      _scheduleHide();
      return KeyEventResult.ignored;
    }

    // Contrôles masqués : raccourcis directs.
    if (_selectKeys.contains(key)) {
      _engine.togglePlay();
      _showControls();
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.arrowLeft) {
      _seekBy(-PlayerScreen.rewindStep);
      _showControls();
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.arrowRight) {
      _seekBy(PlayerScreen.forwardStep);
      _showControls();
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.arrowUp ||
        key == LogicalKeyboardKey.arrowDown) {
      _showControls();
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final reduce = MediaQuery.disableAnimationsOf(context);

    return Scaffold(
      backgroundColor: Colors.black,
      body: Focus(
        // Pas d'autofocus ici : c'est le bouton Lecture/Pause qui l'a au départ.
        // Les touches non traitées par les boutons remontent jusqu'à ce nœud.
        focusNode: _rootFocus,
        onKeyEvent: _onKey,
        child: Stack(
          fit: StackFit.expand,
          children: [
            // La vidéo n'a pas de sens pour le lecteur d'écran.
            ExcludeSemantics(
              child: ValueListenableBuilder<SubtitleStyle>(
                valueListenable: _subtitleStyle,
                builder: (context, style, _) =>
                    _engine.buildVideo(context, subtitleStyle: style),
              ),
            ),
            ValueListenableBuilder<PlaybackState>(
              valueListenable: _engine.state,
              builder: (context, state, _) => _StatusLayer(state: state),
            ),
            // Masqué : contrôles retirés de l'arbre (focus et lecteur d'écran).
            AnimatedOpacity(
              opacity: _controlsVisible ? 1 : 0,
              duration: reduce ? Duration.zero : AppDurations.normal,
              child: _controlsVisible
                  ? ValueListenableBuilder<PlaybackState>(
                      valueListenable: _engine.state,
                      builder: (context, state, _) => _Controls(
                        title: widget.source.title,
                        state: state,
                        playFocus: _playFocus,
                        onTogglePlay: _engine.togglePlay,
                        onRewind: () => _seekBy(-PlayerScreen.rewindStep),
                        onForward: () => _seekBy(PlayerScreen.forwardStep),
                        onTracks: _openTrackPanel,
                      ),
                    )
                  : const SizedBox.shrink(),
            ),
          ],
        ),
      ),
    );
  }
}

/// Chargement, erreur ou fin de lecture, au centre de l'écran.
class _StatusLayer extends StatelessWidget {
  const _StatusLayer({required this.state});

  final PlaybackState state;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final reduce = MediaQuery.disableAnimationsOf(context);

    final String? text;
    if (state.error != null) {
      text = l10n.playerError(state.error!);
    } else if (state.completed) {
      text = l10n.playerEnded;
    } else if (state.buffering) {
      text = l10n.playerLoading;
    } else {
      text = null;
    }
    if (text == null) return const SizedBox.shrink();

    return Center(
      child: Container(
        padding: const EdgeInsets.all(AppSpacing.md),
        decoration: BoxDecoration(
          color: AppColors.background.withValues(alpha: 0.85),
          borderRadius: BorderRadius.circular(AppRadius.md),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Animations désactivées : pas de roue qui tourne, le texte suffit.
            if (state.buffering && state.error == null && !reduce) ...[
              const CircularProgressIndicator(color: AppColors.text),
              const SizedBox(height: AppSpacing.sm),
            ],
            Semantics(
              liveRegion: true,
              child: Text(
                text,
                style: AppText.title.copyWith(
                    color:
                        state.error != null ? AppColors.error : AppColors.text),
                textAlign: TextAlign.center,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Bandeau du bas : titre, barre de progression, boutons.
class _Controls extends StatelessWidget {
  const _Controls({
    required this.title,
    required this.state,
    required this.playFocus,
    required this.onTogglePlay,
    required this.onRewind,
    required this.onForward,
    required this.onTracks,
  });

  final String title;
  final PlaybackState state;
  final FocusNode playFocus;
  final VoidCallback onTogglePlay;
  final VoidCallback onRewind;
  final VoidCallback onForward;
  final VoidCallback onTracks;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final position = formatPlaybackTime(state.position);
    final duration = formatPlaybackTime(state.duration);
    final total = state.duration.inMilliseconds;
    final progress = total > 0
        ? (state.position.inMilliseconds / total).clamp(0.0, 1.0)
        : 0.0;

    return Align(
      alignment: Alignment.bottomCenter,
      child: DecoratedBox(
        // Dégradé sombre : texte blanc lisible quelle que soit l'image.
        decoration: const BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [Color(0x00000000), Color(0xE6000000)],
          ),
        ),
        child: TvSafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(
                AppSpacing.lg, AppSpacing.xl, AppSpacing.lg, AppSpacing.md),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Semantics(
                  header: true,
                  child: Text(title,
                      style: AppText.headline,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis),
                ),
                if (!state.playing && state.error == null && !state.completed)
                  // L'état est écrit en toutes lettres, pas seulement par l'icône.
                  Text(l10n.playerPaused, style: AppText.label),
                const SizedBox(height: AppSpacing.sm),
                Semantics(
                  label: l10n.playerPositionLabel,
                  value: l10n.playerPosition(position, duration),
                  child: ExcludeSemantics(
                    child: Row(
                      children: [
                        Text(position, style: AppText.body),
                        const SizedBox(width: AppSpacing.sm),
                        Expanded(
                          child: ClipRRect(
                            borderRadius: BorderRadius.circular(AppRadius.sm),
                            child: LinearProgressIndicator(
                              value: progress,
                              minHeight: 12,
                              color: AppColors.primary,
                              backgroundColor: AppColors.surfaceHigh,
                            ),
                          ),
                        ),
                        const SizedBox(width: AppSpacing.sm),
                        Text(duration, style: AppText.body),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: AppSpacing.md),
                FocusTraversalGroup(
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      _ControlButton(
                        icon: Icons.replay_10,
                        label: l10n.playerRewind,
                        onTap: onRewind,
                      ),
                      const SizedBox(width: AppSpacing.md),
                      _ControlButton(
                        icon: state.playing ? Icons.pause : Icons.play_arrow,
                        label:
                            state.playing ? l10n.playerPause : l10n.playerPlay,
                        onTap: onTogglePlay,
                        focusNode: playFocus,
                        autofocus: true,
                        large: true,
                      ),
                      const SizedBox(width: AppSpacing.md),
                      _ControlButton(
                        icon: Icons.forward_30,
                        label: l10n.playerForward,
                        onTap: onForward,
                      ),
                      const SizedBox(width: AppSpacing.xl),
                      _ControlButton(
                        icon: Icons.subtitles_outlined,
                        label: l10n.playerTracks,
                        onTap: onTracks,
                      ),
                    ],
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

/// Bouton rond du lecteur : grande icône, focus du design system (contour
/// blanc épais), libellé lu par le lecteur d'écran.
class _ControlButton extends StatelessWidget {
  const _ControlButton({
    required this.icon,
    required this.label,
    required this.onTap,
    this.focusNode,
    this.autofocus = false,
    this.large = false,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final FocusNode? focusNode;
  final bool autofocus;
  final bool large;

  @override
  Widget build(BuildContext context) {
    final size = MediaQuery.textScalerOf(context).scale(large ? 96 : 72);
    return SizedBox(
      width: size,
      height: size,
      child: FocusableCard(
        focusNode: focusNode,
        autofocus: autofocus,
        onTap: onTap,
        borderRadius: 999,
        semanticLabel: label,
        child: Center(
          child: Icon(icon, size: size * 0.55, color: AppColors.text),
        ),
      ),
    );
  }
}
