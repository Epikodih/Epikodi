import 'package:flutter/widgets.dart';

import 'tokens.dart';

/// Ajoute les marges de sécurité (overscan) autour du contenu.
class TvSafeArea extends StatelessWidget {
  const TvSafeArea({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.overscanH,
          vertical: AppSpacing.overscanV,
        ),
        child: child,
      );
}
