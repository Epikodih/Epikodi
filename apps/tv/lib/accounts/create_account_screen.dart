import 'package:design_system/design_system.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:epikodi_client/epikodi_client.dart' show ApiException;

import '../../api/api.dart';
import '../../l10n/app_localizations.dart';
import '../../widgets/language_button.dart';
import '../widgets/widgets.dart';

/// Écran « Créer un compte » (champs de la table USER du schéma) :
/// nom, âge, icône, profil enfant (is_restricted).
/// La langue du profil est celle de l'interface (sélecteur FR/EN de l'en-tête).
/// Le champ `preferences` (thème, texte, sous-titres) est laissé au serveur
/// (valeurs par défaut) : il sera réglable plus tard dans les paramètres.
///
/// Se ferme avec le compte créé (Map) via Navigator.pop, ou `null` si annulé.
/// Le serveur rend le compte actif dès sa création.
class CreateAccountScreen extends StatefulWidget {
  const CreateAccountScreen({super.key, required this.api});

  final Api api;

  @override
  State<CreateAccountScreen> createState() => _CreateAccountScreenState();
}

class _CreateAccountScreenState extends State<CreateAccountScreen> {
  static const _icons = [
    'avatar_01', 'avatar_02', 'avatar_03',
    'avatar_04', 'avatar_05', 'avatar_06',
  ];
  static const _minAge = 3;
  static const _maxAge = 99;
  static const _errorColor = Color(0xFFFF8A80);

  /// En dessous de cette largeur, l'avatar passe au-dessus du formulaire.
  static const _compactWidth = 700.0;

  final _name = TextEditingController();
  String _icon = _icons.first;
  int _age = 18;
  bool _restricted = false;
  bool _busy = false;
  // Message d'erreur déjà traduit (il ne change pas si on rebascule la langue).
  String? _error;
  // Image choisie sur l'ordinateur (remplace l'icône prédéfinie).
  String? _customPath;
  // Survol / focus (télécommande) de la photo cliquable.
  bool _photoHover = false;
  bool _photoFocus = false;

  @override
  void initState() {
    super.initState();
    // Met à jour l'aperçu (initiale) pendant la saisie.
    _name.addListener(() => setState(() {}));
  }

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _back() => Navigator.of(context).pop();

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

  // L'âge et le contrôle parental sont indépendants : la cohérence avec
  // l'âge des médias est vérifiée plus tard, au moment de la lecture.
  void _setRestricted(bool value) {
    setState(() => _restricted = value);
  }

  // Texte d'aide du contrôle parental : doit refléter la règle appliquée
  // par le serveur à la lecture.
  // Classifications d'âge existantes (-10, -12, -16, -18) : avec le contrôle
  // actif, tout ce qui est classé au-dessus de l'âge du profil est bloqué.
  static const _ratings = [10, 12, 16, 18];

  String _restrictedHint(AppLocalizations l10n) {
    if (!_restricted) return l10n.noContentBlocked;
    final blocked = _ratings.where((r) => r > _age);
    if (blocked.isEmpty) return l10n.noContentBlockedAdult;
    return l10n.blocksRatingAndAbove(blocked.first);
  }

  Future<void> _submit() async {
    if (_busy) return;
    final l10n = AppLocalizations.of(context)!;
    final name = _name.text.trim();
    if (name.isEmpty) {
      setState(() => _error = l10n.enterName);
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final profile = await widget.api.createProfile(
        name: name,
        icon: _customPath != null ? 'file:$_customPath' : _icon,
        language: Localizations.localeOf(context).languageCode,
        age: _age,
        isRestricted: _restricted,
      );
      if (mounted) Navigator.of(context).pop(profile);
    } on ApiException catch (e) {
      if (mounted) {
        setState(() {
          _busy = false;
          _error = e.message;
        });
      }
    }
  }

  Color _avatarColor(String icon) {
    final i = _icons.indexOf(icon);
    return HSVColor.fromAHSV(1, (i * 40.0) % 360, 0.55, 0.78).toColor();
  }

  // ---------------------------------------------------------------------
  // Sections
  // ---------------------------------------------------------------------

  /// En-tête fixe : logo, sélecteur de langue de l'app, titre à droite.
  Widget _buildHeader(AppLocalizations l10n, bool compact) {
    return Row(
      children: [
        EpikodiLogo(size: compact ? 40 : 48),
        const SizedBox(width: AppSpacing.md),
        const LanguageButton(),
        const SizedBox(width: AppSpacing.md),
        Expanded(
          child: Text(
            l10n.createAccount,
            style: AppText.headline,
            textAlign: TextAlign.end,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }

  /// Avatar cliquable + choix d'icônes prédéfinies.
  Widget _buildAvatar(
      AppLocalizations l10n, TextScaler scaler, String? initial) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Photo cliquable : ouvre le sélecteur d'image.
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
                      // Petit appareil photo : indique que c'est cliquable.
                      Positioned(
                        right: 0,
                        bottom: 0,
                        child: Container(
                          padding: const EdgeInsets.all(AppSpacing.xs),
                          decoration: BoxDecoration(
                            color: AppColors.surfaceHigh,
                            shape: BoxShape.circle,
                            border:
                                Border.all(color: AppColors.text, width: 2),
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

  /// Nom + réglages (âge, profil enfant).
  Widget _buildForm(AppLocalizations l10n, TextScaler scaler) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(
          controller: _name,
          autofocus: true,
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

        // Âge : FittedBox, le contenu se réduit au lieu de déborder
        // quand la fenêtre est redimensionnée.
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
            onTap: () => _setRestricted(!_restricted),
          ),
        ),
      ],
    );
  }

  /// Zone centrale : avatar + formulaire, centrés verticalement.
  /// Si la fenêtre est trop basse, le contenu défile au lieu de déborder.
  /// Si elle est trop étroite, l'avatar passe au-dessus du formulaire.
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
            // Prend au moins toute la hauteur dispo -> le Center centre.
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

  /// Barre du bas fixe : erreur, puis « Retour » à gauche de « Créer le compte ».
  /// En fenêtre étroite, l'erreur passe au-dessus des boutons.
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
          text: _busy ? l10n.creating : l10n.createAccountButton,
          icon: Icons.check_rounded,
          onTap: _submit,
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
      // « Retour » : Échap (ou la touche Retour d'une télécommande).
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
              // Toute la page (en-tête, contenu, barre du bas) partage la
              // même largeur max et le même centrage : tout est aligné.
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