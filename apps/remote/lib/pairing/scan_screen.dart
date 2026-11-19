import 'package:design_system/design_system.dart';
import 'package:epikodi_client/epikodi_client.dart';
import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../l10n/app_localizations.dart';
import '../theme.dart';
import '../widgets/buttons.dart';

/// Ouvre la caméra et renvoie le texte du QR de la TV.
/// Renvoie [ScanScreen.manualEntry] si l'utilisateur préfère taper le
/// code, `null` s'il revient en arrière.
Future<String?> openScanScreen(BuildContext context) =>
    Navigator.of(context).push<String>(
      MaterialPageRoute(builder: (_) => const ScanScreen()),
    );

class ScanScreen extends StatefulWidget {
  const ScanScreen({super.key});

  /// Résultat renvoyé quand l'utilisateur choisit « Taper le code ».
  static const manualEntry = '';

  @override
  State<ScanScreen> createState() => _ScanScreenState();
}

class _ScanScreenState extends State<ScanScreen> {
  final _controller = MobileScannerController(
    formats: const [BarcodeFormat.qrCode],
    detectionSpeed: DetectionSpeed.noDuplicates,
  );

  bool _done = false;
  bool _notTv = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _onDetect(BarcodeCapture capture) {
    if (_done) return;
    for (final code in capture.barcodes) {
      final value = code.rawValue;
      if (value == null) continue;
      if (PairTarget.fromQr(value) != null) {
        _done = true;
        Navigator.of(context).pop(value);
        return;
      }
      if (!_notTv) setState(() => _notTv = true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        foregroundColor: Colors.white,
        title: Text(l10n.scanTitle, style: RemoteText.title),
      ),
      extendBodyBehindAppBar: true,
      body: MobileScanner(
        controller: _controller,
        onDetect: _onDetect,
        errorBuilder: (context, error) => _ScanError(error: error),
        overlayBuilder: (context, constraints) => _Viewfinder(
          size: constraints.biggest,
          notTv: _notTv ? l10n.scanNotTv : null,
        ),
      ),
    );
  }
}

/// Fond assombri avec une fenêtre carrée bordée du dégradé de marque.
class _Viewfinder extends StatelessWidget {
  const _Viewfinder({required this.size, this.notTv});

  final Size size;
  final String? notTv;

  @override
  Widget build(BuildContext context) {
    final side = (size.shortestSide * 0.7).clamp(180.0, 320.0);
    return Stack(
      children: [
        Positioned.fill(
          child: CustomPaint(painter: _MaskPainter(side: side)),
        ),
        Center(
          child: Container(
            width: side + 8,
            height: side + 8,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(AppRadius.lg + 4),
              gradient: AppBrand.gradient,
            ),
            padding: const EdgeInsets.all(4),
            child: Container(
              decoration: BoxDecoration(
                color: Colors.black,
                borderRadius: BorderRadius.circular(AppRadius.lg),
              ),
              foregroundDecoration: const BoxDecoration(),
            ),
          ),
        ),
        if (notTv != null)
          Positioned(
            left: AppSpacing.md,
            right: AppSpacing.md,
            bottom: AppSpacing.xl,
            child: Semantics(
              liveRegion: true,
              child: Container(
                padding: const EdgeInsets.all(AppSpacing.sm),
                decoration: BoxDecoration(
                  color: AppColors.surface,
                  borderRadius: BorderRadius.circular(AppRadius.md),
                ),
                child: Text(notTv!,
                    style: RemoteText.body, textAlign: TextAlign.center),
              ),
            ),
          ),
      ],
    );
  }
}

class _MaskPainter extends CustomPainter {
  _MaskPainter({required this.side});

  final double side;

  @override
  void paint(Canvas canvas, Size size) {
    final window = RRect.fromRectAndRadius(
      Rect.fromCenter(
          center: size.center(Offset.zero), width: side, height: side),
      const Radius.circular(AppRadius.lg),
    );
    final path = Path()
      ..fillType = PathFillType.evenOdd
      ..addRect(Offset.zero & size)
      ..addRRect(window);
    canvas.drawPath(path, Paint()..color = Colors.black.withAlpha(150));
  }

  @override
  bool shouldRepaint(_MaskPainter old) => old.side != side;
}

/// Caméra refusée ou absente : Gloubi s'inquiète, on propose de taper le code.
class _ScanError extends StatelessWidget {
  const _ScanError({required this.error});

  final MobileScannerException error;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final message = error.errorCode == MobileScannerErrorCode.permissionDenied
        ? l10n.scanNoPermission
        : l10n.scanUnavailable;
    return ColoredBox(
      color: AppColors.background,
      child: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(AppSpacing.md),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Center(
                child: Gloubi(animation: GloubiAnimation.worried, size: 160),
              ),
              const SizedBox(height: AppSpacing.md),
              Text(message,
                  style: RemoteText.body, textAlign: TextAlign.center),
              const SizedBox(height: AppSpacing.lg),
              BrandButton(
                label: l10n.typeCodeInstead,
                icon: Icons.dialpad_rounded,
                onPressed: () =>
                    Navigator.of(context).pop(ScanScreen.manualEntry),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
