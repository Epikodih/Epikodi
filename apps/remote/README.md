# GloubiTV Remote (télécommande Android)

App Android qui se connecte à la TV GloubiTV. Même design que la TV : couleurs,
dégradé, logo et Gloubi (`packages/design_system`).

## Connexion

1. **Serveur** : l'adresse du serveur API est en dur dans `lib/config.dart`
   (`defaultServerAddress`). Au lancement, l'app vérifie qu'il répond
   (`GET /api/health`). L'adresse est affichée en petit en bas de l'écran,
   avec « Changer » pour se connecter à un autre media center (adresse gardée
   ensuite, « Adresse par défaut » pour revenir à celle de la config).
2. **TV** : une fois le serveur joignable, scanner le QR affiché sur la TV ou
   taper le code à 6 chiffres : `POST /api/remote/pair` avec le code et le
   téléphone (`fixtures/pairing/request.json` : nom, `install_id`, `platform`).
   Le QR contient le code (`code=`) : scanner appaire directement, c'est une
   alternative au code tapé (le serveur est déjà connu). Un QR sans code : on
   tape ensuite le code.
3. Le serveur renvoie un token (`fixtures/pairing/response.json`), gardé dans
   le stockage chiffré d'Android.
4. Aux lancements suivants, `GET /api/session` vérifie le token et donne le
   profil actif sur la TV. `unauthorized` ou token expiré : retour à la
   connexion.

5. **Télécommande** : pavé directionnel (zones larges, on peut aussi glisser
   le doigt) + OK, Retour, Accueil. En haut,
   le bouton du profil actif ouvre le même menu déroulant que sur la TV :
   - « Profil » : raccourci vers la page profil de la TV ;
   - « Changer de profil » : liste des comptes (`GET /api/profiles`), PIN si
     besoin, puis `POST /api/session/profile` ;
   - « Appairage » : déconnecte ce téléphone de la TV ;
   - « Infos » : adresse du serveur (modifiable : nouvel appairage), téléphone,
     profil actif, version.

   Les touches et le raccourci « Profil » passent par `lib/remote/tv_commands.dart`,
   sur des routes provisoires du faux serveur (`POST /api/remote/key`,
   `POST /api/remote/shortcut`) : pas encore dans les fixtures. Le faux
   serveur les pousse à la TV (`GET /api/tv/remote`), qui déplace son focus
   comme avec les flèches du clavier (`apps/tv/lib/remote/remote_keys.dart`).
   `GET /dev/remote` montre les dernières touches reçues.

6. **Clavier** : quand un champ texte prend le focus sur la TV (ex. le nom à
   la création de compte), la TV l'annonce (`POST /api/tv/text-input`), le
   téléphone le reçoit (`GET /api/remote/events`) et ouvre son clavier. Chaque
   lettre part sur la TV (`POST /api/remote/text`), « Valider » fait comme
   Entrée. Le clavier se ferme quand la TV quitte le champ ; OK sur le champ
   le rouvre. Routes provisoires du faux serveur, comme les touches.

Gloubi : bonjour à l'arrivée, inquiet (en boucle) quand le serveur ne répond
pas ou que le code est refusé, coucou une fois connecté. Avec les animations
désactivées dans Android, il reste en image fixe.

## Lancer

```bash
just remote
```

Avec le faux serveur (`python test_api.py`) sur le PC : mettre l'IP Wi-Fi du
PC dans `defaultServerAddress` (ou « Changer » dans l'app). Le téléphone et le
PC doivent être sur le même Wi-Fi, et le pare-feu Windows doit laisser passer
Python. Dans l'émulateur Android, le PC est `10.0.2.2`.

## Code

| Dossier | Contenu |
|---|---|
| `lib/pairing/` | connexion au serveur, adresse, code ou QR, caméra |
| `lib/connected/` | télécommande (vérifie le token, croix, Retour, Accueil) |
| `lib/remote/` | croix directionnelle, menu du profil, changer de compte, commandes TV |
| `lib/storage/` | TV appairée et `install_id` |
| `lib/widgets/` | boutons, page, saisie du code en tuiles |
| `lib/deps.dart` | ce que les tests remplacent (serveur, stockage, caméra) |

Les routes (`client.remote.pair`, `client.remote.session`) et la lecture du
QR (`PairTarget`) sont dans `packages/client`, partagées avec la TV.

## Tests

```bash
just test-remote
```
