import 'package:epikodi_client/epikodi_client.dart' show ApiException;
import 'package:design_system/design_system.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../api/api.dart';
import '../l10n/app_localizations.dart';
import '../widgets/language_button.dart';
import '../widgets/pin_dialog.dart';
import '../widgets/widgets.dart';

/// Page du profil actif : nom, âge, icône/photo, langue et profil enfant.
/// Toucher à la restriction d'âge demande le code PIN (vérifié par le serveur).
///
/// Se ferme avec le profil mis à jour (Map) via Navigator.pop,
/// ou `null` si rien n'a été enregistré.
class ProfileScreen extends StatefulWidget {
  const ProfileScreen({super.key, required this.api, required this.profile});

  final Api api;

  /// Compte actif (réponse de POST /api/session/profile).
  final Map<String, dynamic> profile;

  @override
  State<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends State<ProfileScreen> {
  static const _icons = [
    'avatar_01',
    'avatar_02',
    'avatar_03',
    'avatar_04',
    'avatar_05',
    'avatar_06',
  ];
  static const _minAge = 3;
  static const _maxAge = 99;
  static const _ratings = [10, 12, 16, 18];
  static const _errorColor = Color(0xFFFF8A80);

  /// En dessous de cette largeur, l'avatar passe au-dessus du formulaire.
  static const _compactWidth = 700.0;

  late final TextEditingController _name;
  late final String _initialName;
  late final int _initialAge;
  late final bool _initialRestricted;
  late final String _initialLanguage;
  late final String _initialIcon;

  late int _age;
  late bool _restricted;
  late String _icon;
  String? _customPath;
  bool _photoHover = false;
  bool _photoFocus = false;
  bool _busy = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _initialName = '${widget.profile['name'] ?? ''}';
    final age = widget.profile['age'];
    _initialAge = age is int ? age.clamp(_minAge, _maxAge) : 18;
    _initialRestricted = widget.profile['is_restricted'] == true;
    _initialLanguage = '${widget.profile['language'] ?? ''}';

    final rawIcon = '${widget.profile['icon'] ?? ''}';
    if (rawIcon.startsWith('file:')) {
      _customPath = rawIcon.substring(5);
      _icon = _icons.first;
      _initialIcon = rawIcon;
    } else {
      _icon = _icons.contains(rawIcon) ? rawIcon : _icons.first;
      _initialIcon = _icon;
    }

    _name = TextEditingController(text: _initialName)
      ..addListener(() => setState(() {}));
    _age = _initialAge;
    _restricted = _initialRestricted;
  }

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _back() => Navigator.of(context).pop();

  String get _iconValue => _customPath != null ? 'file:$_customPath' : _icon;

  Future<void> _pickImage() async {
    try {
      final res = await FilePicker.pickFiles(type: FileType.image);
      final path = res.firstOrNull?.path;
      if (path != null) setState(() => _customPath = path);
    } catch (_) {
      if (!mounted) return;
      setState(() => _error = AppLocalizations.of(context)!.imagePickError);
    }
  }

  void _setAge(int delta) {
    setState(() => _age = (_age + delta).clamp(_minAge, _maxAge));
  }

  bool get _changed =>
      _name.text.trim() != _initialName ||
      _age != _initialAge ||
      _restricted != _initialRestricted ||
      _iconValue != _initialIcon ||
      Localizations.localeOf(context).languageCode != _initialLanguage;

  /// Le PIN protège la restriction d'âge (pas l'icône ni le nom).
  bool get _needsPin =>
      _restricted != _initialRestricted ||
      (_age != _initialAge && (_restricted || _initialRestricted));

  String _restrictedHint(AppLocalizations l10n) {
    if (!_restricted) return l10n.noContentBlocked;
    final blocked = _ratings.where((r) => r > _age);
    if (blocked.isEmpty) return l10n.noContentBlockedAdult;
    return l10n.blocksRatingAndAbove(blocked.first);
  }

  Color _avatarColor(String icon) {
    final i = _icons.indexOf(icon);
    return HSVColor.fromAHSV(1, (i * 40.0) % 360, 0.55, 0.78).toColor();
  }

  Future<void> _save() async {
    if (_busy) return;
    final l10n = AppLocalizations.of(context)!;
    final name = _name.text.trim();
    if (name.isEmpty) {
      setState(() => _error = l10n.enterName);
      return;
    }
    if (!_changed) {
      _back();
      return;
    }

    String? pin;
    String? pinError;
    while (true) {
      if (_needsPin) {
        if (!mounted) return;
        pin = await showPinDialog(context, errorText: pinError);
        if (pin == null || !mounted) return;
      }

      setState(() {
        _busy = true;
        _error = null;
      });
      try {
        final updated = await widget.api.updateProfile(
          widget.profile['id'],
          name: name,
          icon: _iconValue, // <-- à ajouter dans Api.updateProfile
          age: _age,
          isRestricted: _restricted,
          language: Localizations.localeOf(context).languageCode,
          pin: pin,
        );
        if (mounted) Navigator.of(context).pop(updated);
        return;
      } on ApiException catch (e) {
        if (!mounted) return;
        setState(() => _busy = false);
        if (e.code == 'pin_invalid') {
          pinError = l10n.pinWrong;
          continue;
        }
        setState(() => _error = e.message);
        return;
      }
    }
  }

  // ---------------------------------------------------------------------
  // Sections
  // ---------------------------------------------------------------------

  Widget _buildHeader(AppLocalizations l10n, bool compact) {
    return Row(
      children: [
        EpikodiLogo(size: compact ? 40 : 48),
        const SizedBox(width: AppSpacing.md),
        const LanguageButton(),
        const SizedBox(width: AppSpacing.md),
        Expanded(
          child: Text(
            l10n.profile,
            style: AppText.headline,
            textAlign: TextAlign.end,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }

  /// Avatar cliquable (photo) + choix de couleurs/icônes prédéfinies.
  Widget _buildAvatar(
      AppLocalizations l10n, TextScaler scaler, String? initial) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Semantics(
          button: true,
          label: l10n.choosePhoto,
          child: Material(
            type: MaterialType.transparency,
            child: InkWell(
              customBorder: const CircleBorder(),
              onTap: _pickImage,
              onHover: (v) => setState(() => _photoHover = v),
              onFocusChange: (v) => setState(() => _photoFocus = v),
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 120),
                  padding: const EdgeInsets.all(4),
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    border: Border.all(
                      color: (_photoFocus || _photoHover)
                          ? AppColors.text
                          : Colors.transparent,
                      width: 3,
                    ),
                  ),
                  child: Stack(
                    children: [
                      AvatarPreview(
                        color: _avatarColor(_icon),
                        initial: initial,
                        imagePath: _customPath,
                        size: scaler.scale(120),
                      ),
                      Positioned(
                        right: 0,
                        bottom: 0,
                        child: Container(
                          padding: const EdgeInsets.all(AppSpacing.xs),
                          decoration: BoxDecoration(
                            color: AppColors.surfaceHigh,
                            shape: BoxShape.circle,
                            border: Border.all(color: AppColors.text, width: 2),
                          ),
                          child: Icon(
                            Icons.photo_camera_rounded,
                            size: 20,
                            color: AppColors.text,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
        const SizedBox(height: AppSpacing.md),
        Wrap(
          alignment: WrapAlignment.center,
          spacing: AppSpacing.sm,
          runSpacing: AppSpacing.sm,
          children: [
            for (var i = 0; i < _icons.length; i++)
              AvatarChoice(
                color: _avatarColor(_icons[i]),
                selected: _customPath == null && _icon == _icons[i],
                semantic: _icon == _icons[i]
                    ? l10n.avatarIconSelected(i + 1)
                    : l10n.avatarIcon(i + 1),
                onTap: () => setState(() {
                  _icon = _icons[i];
                  _customPath = null;
                }),
              ),
          ],
        ),
      ],
    );
  }

  Widget _buildForm(AppLocalizations l10n, TextScaler scaler) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(
          controller: _name,
          maxLength: 20,
          textInputAction: TextInputAction.done,
          style: AppText.title,
          onSubmitted: (_) => FocusScope.of(context).nextFocus(),
          decoration: InputDecoration(
            labelText: l10n.name,
            hintText: l10n.nameHint,
            counterText: '',
            labelStyle: AppText.body.copyWith(color: AppColors.textMuted),
            filled: true,
            fillColor: AppColors.surfaceHigh,
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(AppRadius.lg),
              borderSide: BorderSide.none,
            ),
            focusedBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(AppRadius.lg),
              borderSide: BorderSide(color: AppColors.text, width: 3),
            ),
          ),
        ),
        const SizedBox(height: AppSpacing.md),
        SettingCard(
          label: l10n.age,
          child: FittedBox(
            fit: BoxFit.scaleDown,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                RoundButton(
                  icon: Icons.remove_rounded,
                  semantic: l10n.decreaseAge,
                  onTap: () => _setAge(-1),
                ),
                SizedBox(
                  width: scaler.scale(64),
                  child: Text(
                    l10n.ageYears(_age),
                    style: AppText.title,
                    textAlign: TextAlign.center,
                    maxLines: 1,
                  ),
                ),
                RoundButton(
                  icon: Icons.add_rounded,
                  semantic: l10n.increaseAge,
                  onTap: () => _setAge(1),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: AppSpacing.sm),
        SettingRow(
          label: l10n.childProfile,
          hint: _restrictedHint(l10n),
          child: PillChoice(
            text: _restricted ? l10n.yes : l10n.no,
            selected: _restricted,
            semanticPrefix: l10n.childProfileSemantic,
            compact: true,
            onTap: () => setState(() => _restricted = !_restricted),
          ),
        ),
        if (_needsPin) ...[
          const SizedBox(height: AppSpacing.sm),
          Row(
            children: [
              Icon(
                Icons.lock_rounded,
                size: scaler.scale(28),
                color: AppColors.textMuted,
              ),
              const SizedBox(width: AppSpacing.xs),
              Expanded(
                child: Text(
                  l10n.pinWillBeAsked,
                  style: AppText.body.copyWith(color: AppColors.textMuted),
                ),
              ),
            ],
          ),
        ],
      ],
    );
  }

  Widget _buildBody(
      AppLocalizations l10n, TextScaler scaler, String? initial, bool compact) {
    return LayoutBuilder(
      builder: (context, box) {
        final avatar = SizedBox(
          width: scaler.scale(250),
          child: _buildAvatar(l10n, scaler, initial),
        );
        final form = _buildForm(l10n, scaler);

        return SingleChildScrollView(
          child: ConstrainedBox(
            constraints: BoxConstraints(minHeight: box.maxHeight),
            child: Center(
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: AppSpacing.xs),
                child: compact
                    ? Column(
                        mainAxisSize: MainAxisSize.min,
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          Center(child: avatar),
                          const SizedBox(height: AppSpacing.md),
                          form,
                        ],
                      )
                    : Row(
                        crossAxisAlignment: CrossAxisAlignment.center,
                        children: [
                          avatar,
                          const SizedBox(width: AppSpacing.lg),
                          Expanded(child: form),
                        ],
                      ),
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _buildBottomBar(AppLocalizations l10n, bool compact) {
    final error = _error == null
        ? null
        : Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.error_outline_rounded, color: _errorColor),
              const SizedBox(width: AppSpacing.xs),
              Flexible(
                child: Text(
                  _error!,
                  style: AppText.body.copyWith(color: _errorColor),
                ),
              ),
            ],
          );

    final buttons = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        PillButton(
          text: l10n.back,
          icon: Icons.arrow_back_rounded,
          onTap: _back,
        ),
        const SizedBox(width: AppSpacing.sm),
        PillButton(
          text: _busy ? l10n.saving : l10n.save,
          icon: Icons.check_rounded,
          onTap: _save,
        ),
      ],
    );

    if (compact) {
      return Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (error != null) ...[
            error,
            const SizedBox(height: AppSpacing.sm),
          ],
          Align(
            alignment: Alignment.centerRight,
            child: FittedBox(fit: BoxFit.scaleDown, child: buttons),
          ),
        ],
      );
    }

    // Les boutons gardent leur taille, mais se réduisent (comme en mode
    // compact) si le texte agrandi ne tient plus sur la largeur.
    return LayoutBuilder(
      builder: (context, box) => Row(
        children: [
          Expanded(child: error ?? const SizedBox.shrink()),
          const SizedBox(width: AppSpacing.md),
          ConstrainedBox(
            constraints: BoxConstraints(maxWidth: box.maxWidth - AppSpacing.md),
            child: FittedBox(fit: BoxFit.scaleDown, child: buttons),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final scaler = MediaQuery.textScalerOf(context);
    final initial = _name.text.trim().isEmpty
        ? null
        : _name.text.trim().characters.first.toUpperCase();

    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): _back,
        const SingleActivator(LogicalKeyboardKey.goBack): _back,
      },
      child: Scaffold(
        body: BrandBackground(
          child: TvSafeArea(
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: AppSpacing.lg,
                vertical: AppSpacing.md,
              ),
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 1000),
                  child: LayoutBuilder(
                    builder: (context, box) {
                      final compact = box.maxWidth < _compactWidth;
                      return Column(
                        children: [
                          _buildHeader(l10n, compact),
                          const SizedBox(height: AppSpacing.md),
                          Expanded(
                            child: _buildBody(l10n, scaler, initial, compact),
                          ),
                          const SizedBox(height: AppSpacing.md),
                          _buildBottomBar(l10n, compact),
                        ],
                      );
                    },
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
