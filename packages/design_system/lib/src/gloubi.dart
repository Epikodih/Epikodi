import 'dart:async';

import 'package:flutter/widgets.dart';

const _package = 'design_system';
const _dir = 'assets/mascot';

/// Animations available for Gloubi.
enum GloubiAnimation {
  /// Rises out of the ground (1.33 s, plays once). Ends on the first frame of [idle].
  arrival,

  /// Waiting: breathes, blinks, looks up (4 s, loops).
  idle,

  /// Waves hello (1.63 s, plays once).
  wave,

  /// Worried, looks around (2.07 s, plays once). Errors, lost connection...
  worried,
}

/// Still image of Gloubi (smiling, eye centered).
const AssetImage gloubiImage = AssetImage('$_dir/mascot.png', package: _package);

AssetImage _asset(GloubiAnimation a) =>
    AssetImage('$_dir/${a.name}.webp', package: _package);

/// Displays Gloubi.
///
/// - `Gloubi()`: still image.
/// - `Gloubi(animation: GloubiAnimation.wave)`: a single animation.
/// - `Gloubi.entrance()`: `arrival` once, then `idle` looping, with no jump.
class Gloubi extends StatefulWidget {
  const Gloubi({super.key, this.animation, this.size = 256})
      : _entrance = false;

  const Gloubi.entrance({super.key, this.size = 256})
      : animation = GloubiAnimation.arrival,
        _entrance = true;

  /// `null` shows the still image.
  final GloubiAnimation? animation;
  final double size;
  final bool _entrance;

  @override
  State<Gloubi> createState() => _GloubiState();
}

class _GloubiState extends State<Gloubi> {
  static const _arrivalDuration = Duration(milliseconds: 1333);

  GloubiAnimation? _current;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _current = widget.animation;
    if (widget._entrance) {
      _timer = Timer(_arrivalDuration, () {
        if (mounted) setState(() => _current = GloubiAnimation.idle);
      });
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (widget._entrance) {
      // Preload idle so the switch from arrival does not flash.
      precacheImage(_asset(GloubiAnimation.idle), context);
    }
  }

  @override
  void didUpdateWidget(Gloubi old) {
    super.didUpdateWidget(old);
    if (!widget._entrance && old.animation != widget.animation) {
      _current = widget.animation;
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final current = _current;
    return Image(
      image: current == null ? gloubiImage : _asset(current),
      width: widget.size,
      height: widget.size,
      gaplessPlayback: true,
      semanticLabel: 'Gloubi',
    );
  }
}
