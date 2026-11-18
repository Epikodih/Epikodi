import 'dart:async';

import 'package:flutter/widgets.dart';

const _package = 'design_system';
const _dir = 'assets/mascot';

/// Animations available for Gloubi.
enum GloubiAnimation {
  /// Rises out of the ground (1.33 s, plays once). Ends on the first frame of [idle].
  arrival(Duration(milliseconds: 1333)),

  /// Waiting: breathes, blinks, looks up (4 s, loops).
  idle(Duration(seconds: 4)),

  /// Waves hello (1.63 s, plays once).
  wave(Duration(milliseconds: 1630)),

  /// Worried, looks around (2.07 s, loops). Errors, lost connection...
  worried(Duration(milliseconds: 2070));

  const GloubiAnimation(this.duration);

  /// Length of one play of the animation.
  final Duration duration;
}

/// Still image of Gloubi (smiling, eye centered).
const AssetImage gloubiImage =
    AssetImage('$_dir/mascot.png', package: _package);

/// Still image of Gloubi worried (for « animations réduites »).
const AssetImage gloubiWorriedImage =
    AssetImage('$_dir/worried.png', package: _package);

AssetImage _asset(GloubiAnimation a) =>
    AssetImage('$_dir/${a.name}.webp', package: _package);

/// Displays Gloubi.
///
/// - `Gloubi()`: still image.
/// - `Gloubi(animation: GloubiAnimation.worried)`: a single animation.
/// - `Gloubi.entrance()`: `arrival`, then `idle` looping.
/// - `Gloubi.hello()`: `wave`, then `idle` looping.
/// - `Gloubi.greeting()`: `arrival`, `wave`, then `idle` looping (startup).
///
/// With animations disabled (`MediaQuery.disableAnimations`, the
/// « animations réduites » setting), shows a still image instead:
/// worried for [GloubiAnimation.worried], smiling otherwise.
class Gloubi extends StatefulWidget {
  const Gloubi({super.key, this.animation, this.size = 256})
      : _steps = const [];

  const Gloubi.entrance({super.key, this.size = 256})
      : animation = GloubiAnimation.idle,
        _steps = const [GloubiAnimation.arrival];

  const Gloubi.hello({super.key, this.size = 256})
      : animation = GloubiAnimation.idle,
        _steps = const [GloubiAnimation.wave];

  const Gloubi.greeting({super.key, this.size = 256})
      : animation = GloubiAnimation.idle,
        _steps = const [GloubiAnimation.arrival, GloubiAnimation.wave];

  /// Animation shown at the end (after the steps, if any).
  /// `null` shows the still image.
  final GloubiAnimation? animation;
  final double size;

  /// Animations played once, in order, before [animation].
  final List<GloubiAnimation> _steps;

  /// Total time of the steps played before [animation] (zero if none).
  /// Lets a screen wait for the end of a greeting.
  static Duration introDuration(List<GloubiAnimation> steps) =>
      steps.fold(Duration.zero, (sum, a) => sum + a.duration);

  /// Length of [Gloubi.greeting] before it settles on `idle`.
  static final Duration greetingDuration =
      introDuration(const [GloubiAnimation.arrival, GloubiAnimation.wave]);

  @override
  State<Gloubi> createState() => _GloubiState();
}

class _GloubiState extends State<Gloubi> {
  GloubiAnimation? _current;
  int _step = 0;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _startSteps();
  }

  void _startSteps() {
    _timer?.cancel();
    _step = 0;
    if (widget._steps.isEmpty) {
      _current = widget.animation;
      return;
    }
    _current = widget._steps.first;
    _scheduleNext();
  }

  void _scheduleNext() {
    _timer = Timer(widget._steps[_step].duration, () {
      if (!mounted) return;
      setState(() {
        _step++;
        _current = _step < widget._steps.length
            ? widget._steps[_step]
            : widget.animation;
      });
      if (_step < widget._steps.length) _scheduleNext();
    });
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // Preload what comes next so each switch does not flash.
    final next = {...widget._steps.skip(1)};
    if (widget.animation != null) next.add(widget.animation!);
    for (final a in next) {
      precacheImage(_asset(a), context);
    }
  }

  @override
  void didUpdateWidget(Gloubi old) {
    super.didUpdateWidget(old);
    if (old.animation != widget.animation || old._steps != widget._steps) {
      _startSteps();
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  ImageProvider get _image {
    final current = _current;
    final reduced = MediaQuery.maybeDisableAnimationsOf(context) ?? false;
    if (reduced || current == null) {
      final worried = widget.animation == GloubiAnimation.worried ||
          current == GloubiAnimation.worried;
      return worried ? gloubiWorriedImage : gloubiImage;
    }
    return _asset(current);
  }

  @override
  Widget build(BuildContext context) {
    return Image(
      image: _image,
      width: widget.size,
      height: widget.size,
      fit: BoxFit.contain,
      gaplessPlayback: true,
      semanticLabel: 'Gloubi',
    );
  }
}
