# Processus de démarrage et d'appairage : serveur Rust, app TV et télécommande

Document technique décrivant, de bout en bout, le lancement du serveur Rust et de l'application TV (Flutter), puis l'appairage de la télécommande Android, l'ouverture de session et le choix du profil.

## 1. Vue d'ensemble

Le système repose sur trois composants :

- **Le serveur Rust** : cœur du système. Il expose l'API HTTP sur le port 7878, gère la base de données, génère les codes d'appairage et émet les tokens de session.
- **L'application TV (Flutter)** : écran principal du salon. Elle est lancée en même temps que le serveur et s'y connecte en local.
- **L'application télécommande (Flutter, Android)** : lancée sur un téléphone, elle se connecte au serveur via son adresse IP sur le réseau local, s'appaire avec la TV et pilote ensuite l'application.

### Chronologie résumée

1. Lancement simultané du serveur Rust et de l'app TV.
2. Le serveur initialise la base de données (ou la crée si elle n'existe pas) et s'y connecte.
3. Le serveur génère le code d'appairage et le QR code, et les garde en mémoire.
4. L'app TV se connecte au serveur sur http://localhost:7878.
5. Une fois la base prête, le serveur lui renvoie dans la foulée le code et le QR code, et la TV les affiche.
6. La télécommande Android atteint l'API du serveur via son adresse IP.
7. L'utilisateur scanne le QR code puis saisit le code d'appairage.
8. Le serveur valide l'appairage et renvoie un token de session.
9. La télécommande affiche **Invité**, la liste des comptes (ou aucun si premier lancement) et **Créer un compte**.
10. L'utilisateur choisit un profil ; le serveur l'associe à la session et la TV affiche l'interface correspondante.

## 2. Prérequis et hypothèses

- Le serveur, la TV et le téléphone sont sur le même réseau local (Wi-Fi/Ethernet).
- L'app TV tourne sur la même machine que le serveur, d'où l'usage de localhost.
- Le port 7878 est libre sur la machine et autorisé par le pare-feu pour les connexions entrantes du réseau local (nécessaire pour la télécommande).
- Le serveur écoute sur une interface accessible depuis le réseau (par exemple 0.0.0.0:7878) et pas uniquement sur 127.0.0.1, sinon le téléphone ne pourra pas l'atteindre.

## 3. Phase 1 : démarrage du serveur et de l'app TV

### 3.1 Lancement simultané

Le serveur Rust et l'app TV démarrent en même temps. Comme leurs temps de démarrage diffèrent, l'app TV peut être prête avant que le serveur n'écoute. Elle ne doit donc pas considérer un premier échec de connexion comme définitif.

Comportement attendu côté TV :

- Afficher un écran de chargement (« Démarrage du serveur… »).
- Tenter de joindre http://localhost:7878 à intervalles réguliers (par exemple toutes les 500 ms à 1 s, avec un délai maximal).
- Passer à l'étape suivante dès qu'une réponse valide arrive (par exemple via une route de santé de type /health).
- Si le délai maximal est dépassé, afficher une erreur claire avec une action « Réessayer ».

### 3.2 Connexion de la TV au serveur

L'app TV ouvre sa connexion vers http://localhost:7878. À ce moment, la base est déjà prête et le code d'appairage existe déjà (phases 2 et 3). Cette connexion sert à deux choses :

- Interroger l'API (état du serveur, état de l'appairage, etc.).
- Recevoir dans la réponse à sa connexion le code d'appairage et le QR code, sans avoir à les redemander.

Le serveur n'a donc pas besoin de pousser le code vers la TV : un simple échange requête/réponse suffit pour cette étape.

## 4. Phase 2 : initialisation de la base de données

Au démarrage, le serveur prépare son stockage avant d'accepter du trafic utile.

Étapes :

1. Vérifier si le fichier ou la base de données existe à l'emplacement prévu.
2. S'il n'existe pas : créer la base.
3. Appliquer le schéma (création des tables, migrations éventuelles) pour que la structure soit à jour.
4. Vérifier que la base est utilisable (ouverture réussie, écriture possible).
5. En cas d'échec, journaliser l'erreur et arrêter le démarrage proprement plutôt que de lancer un serveur à moitié fonctionnel.

Données concernées (d'après le schéma) : profils (USER), appareils appairés (DEVICE, avec token\_hash), état global (APP\_STATE), sources, médias, favoris, listes de lecture, file d'attente et extensions.

**Code d'appairage** : il n'est pas stocké en base, et le schéma ne contient d'ailleurs aucune table pour lui. Il est gardé en mémoire dans le serveur, avec son expiration et son compteur de tentatives. Si le serveur redémarre, la TV redémarre aussi et affiche un nouveau code. Seul le résultat de l'appairage est écrit en base (DEVICE, avec token\_hash).

**Règle** : le serveur ne renvoie rien à la TV tant qu'il n'est pas connecté à la base. Tant que l'initialisation n'est pas terminée, il répond « pas prêt » (par exemple HTTP 503) et la TV réessaie. C'est ce signal que la TV attend à l'étape 3.1.

## 5. Phase 3 : génération du code d'appairage et du QR code

Au démarrage, une fois la base prête, le serveur génère les éléments d'appairage et les garde en mémoire. Il les renvoie à la TV dès qu'elle se connecte, à condition que la base soit prête.

### 5.1 Code d'appairage

- Code court, lisible et facile à saisir sur un téléphone (par exemple 6 chiffres).
- Généré avec un générateur aléatoire cryptographiquement sûr, pas un compteur ni un aléa prévisible.
- Stocké côté serveur avec : sa date de création, sa durée de validité, son état (non utilisé / utilisé) et un compteur de tentatives.
- À usage unique et à durée de vie limitée (par exemple quelques minutes). À l'expiration, un nouveau code est généré et la TV met son affichage à jour.

### 5.2 QR code

Le QR code encode les informations nécessaires à la télécommande pour atteindre le serveur, typiquement :

- L'adresse IP du serveur sur le réseau local et le port (7878) ;
- Éventuellement un identifiant de session d'appairage.

Il est préférable que le QR code ne contienne pas le code d'appairage lui-même : le QR sert à localiser le serveur, le code prouve que l'utilisateur est physiquement devant la TV. Les deux facteurs sont complémentaires.

### 5.3 Envoi à la TV

Le serveur renvoie le code et le QR code (ou les données pour le générer) à la TV dans la réponse à sa connexion. La TV les affiche avec des instructions claires : « Scannez le QR code avec l'application télécommande, puis saisissez le code ».

Si le code expire ou est consommé, le serveur informe la TV pour qu'elle rafraîchisse l'affichage.

## 6. Phase 4 : connexion de la télécommande Android

### 6.1 Atteindre le serveur

L'app télécommande doit connaître l'adresse du serveur : http://\<IP-du-serveur>:7878/api. Deux cas :

- **Via le QR code** : le scan fournit directement l'IP et le port. C'est le parcours normal.
- **Saisie manuelle** : option de secours si la caméra n'est pas disponible ou si le QR code est illisible.

Après le scan, la télécommande vérifie que le serveur répond (route de santé ou de version) avant de demander le code.

### 6.2 Saisie du code et demande d'appairage

1. L'utilisateur lit le code affiché sur la TV et le saisit dans la télécommande.
2. La télécommande envoie une requête d'appairage au serveur contenant le code et des informations sur l'appareil (nom, identifiant unique de l'installation, etc.).
3. Le serveur vérifie :
   - que le code existe et correspond à la session en cours ;
   - qu'il n'est pas expiré ni déjà utilisé ;
   - que le nombre de tentatives n'est pas dépassé.
4. En cas de succès, le code est invalidé immédiatement (usage unique).
5. En cas d'échec, le serveur renvoie une erreur explicite (code invalide, expiré, trop de tentatives) et la télécommande l'affiche.

### 6.3 Protection contre les abus

- Limiter le nombre d'essais par code (par exemple 5) puis invalider le code et en générer un nouveau.
- Ajouter un léger délai croissant après chaque échec.
- Un code à 6 chiffres n'offre que 1 000 000 de combinaisons : c'est la limitation des tentatives qui assure la sécurité, pas la taille du code seule.

## 7. Phase 5 : émission du token de session

Quand l'appairage est validé, le serveur renvoie à la télécommande un token de session. Son rôle : garantir que les requêtes suivantes viennent bien d'un utilisateur connecté et appairé.

Contenu de la réponse :

- Le token de session ;
- Sa durée de validité (si applicable) ;
- Éventuellement l'identifiant de l'appareil enregistré.

Bonnes pratiques :

- Token généré aléatoirement (opaque) ou JWT signé, avec une expiration.
- Côté serveur, associer le token à l'appareil appairé pour pouvoir le révoquer (déconnexion, appareil perdu).
- Côté télécommande, le conserver dans un stockage sécurisé (Keystore Android, via flutter\_secure\_storage par exemple).
- Toutes les requêtes suivantes envoient le token dans l'en-tête Authorization: Bearer \<token>.
- Le serveur rejette avec 401 toute requête sans token valide ; la télécommande retourne alors à l'écran d'appairage.

À réception du token, le serveur peut aussi notifier la TV que l'appairage a réussi, pour qu'elle quitte l'écran « code et QR » et passe à l'étape suivante.

## 8. Phase 6 : choix du profil

Une fois la télécommande authentifiée par son token, le serveur lui renvoie la liste des profils. La télécommande affiche trois possibilités :

| Choix | Comportement |
| --- | --- |
| **Invité** | Accès immédiat, sans code. Le profil est **restreint par défaut**. Rust enregistre dans la session que l'utilisateur est invité et applique les restrictions à chaque requête. Aucun compte n'est créé en base. |
| **Compte existant** | Si le compte est **restreint**, il s'ouvre directement. S'il est **non restreint**, l'utilisateur saisit son **code PIN à 4 chiffres** avant d'entrer. |
| **Créer un compte** | L'utilisateur choisit son **nom** et son niveau de **restriction** (restreint ou non). S'il choisit « non restreint », il définit aussi son PIN à 4 chiffres. |

Au premier lancement (aucun compte), la liste est vide : seuls **Invité** et **Créer un compte** sont proposés.

Après le choix, le serveur associe le profil (invité ou compte) à la session de l'appareil, puis la TV affiche l'interface correspondante. L'appareil reste appairé : on peut changer de profil sans refaire l'appairage.

**Règles côté serveur :**

- Le statut (invité, restreint, non restreint) est toujours vérifié par Rust, jamais déduit de ce que la télécommande affiche.
- Le PIN est stocké haché, avec un nombre de tentatives limité (4 chiffres = seulement 10 000 combinaisons), comme pour le code d'appairage.
- Un invité ne peut ni passer en non restreint, ni modifier les comptes.

## 9. Diagramme de séquence (texte)

```
Serveur Rust        App TV (Flutter)        Télécommande (Android)
    |                     |                        |
    |<--- lancement simultané --->                 |
    |                     |                        |
    | init / création DB  |                        |
    | connexion DB OK     |                        |
    | génère code + QR    |                        |
    | (gardés en mémoire) |                        |
    |                     |                        |
    |<-- connexion localhost:7878 --|              |
    |  (503 tant que la DB n'est pas prête)        |
    |--- réponse : code + QR -->|                  |
    |                     | affiche code + QR      |
    |                     |                        |
    |<----------- accès http://IP:7878/api --------|
    |<----------- scan QR + saisie du code --------|
    | vérifie le code     |                        |
    |------------ token de session --------------->|
    |--- notifie : appairage réussi -->|           |
    |                     |                        |
    |------ profils : Invité + comptes + création ->|
    |<-- choix : invité | compte (+PIN si non restreint) | nouveau compte --|
    | enregistre le profil dans la session         |
    |--- notifie : profil actif -->|               |
```

## 10. Cas d'erreur à prévoir

| Situation | Comportement recommandé |
| --- | --- |
| Serveur pas encore prêt au lancement de la TV | La TV réessaie en boucle avec écran de chargement |
| Échec de création ou d'ouverture de la DB | Le serveur journalise et s'arrête avec un message explicite |
| Port 7878 déjà utilisé | Le serveur échoue au démarrage avec une erreur claire |
| Téléphone non joignable (autre réseau, pare-feu) | La télécommande affiche « Serveur introuvable » et propose la saisie manuelle de l'IP |
| Code d'appairage invalide | Erreur explicite, tentative comptée |
| Code d'appairage expiré | Erreur explicite, la TV affiche déjà un nouveau code |
| Trop de tentatives sur le code | Code invalidé, nouveau code généré |
| PIN de compte incorrect | Erreur explicite, tentative comptée ; trop d'essais : verrouillage temporaire |
| Token expiré ou révoqué (401) | La télécommande retourne à l'écran d'appairage |
| TV déconnectée pendant l'appairage | Le serveur conserve l'état et renvoie le code à la reconnexion de la TV |
| Perte du réseau côté télécommande | Reconnexion automatique avec le token existant, sans nouvel appairage |

## 11. Points de sécurité à retenir

- Le trafic reste en HTTP sur le réseau local : le token circule en clair sur ce réseau. Si le niveau de sécurité l'exige, envisager du HTTPS avec un certificat local.
- Code d'appairage : aléatoire sûr, usage unique, expiration courte, tentatives limitées.
- Token : expiration, révocation possible, stockage sécurisé sur le téléphone.
- Profils : le statut invité/restreint est tenu et vérifié par Rust ; les comptes non restreints sont protégés par un PIN à 4 chiffres haché, à tentatives limitées.
- Les routes sensibles (comptes, lecture, réglages) exigent toutes un token valide ; seules la santé du serveur et l'appairage sont publiques.
- L'accès localhost de la TV peut être traité comme de confiance, mais uniquement pour les routes qui lui sont propres.

## 12. Points à préciser

Ces éléments ne figurent pas dans la description d'origine et devront être complétés ou validés :

- Comment la TV apprend que l'appairage a réussi (interrogation régulière ou notification poussée).
- La durée de validité du code et le nombre maximal de tentatives.
- Le format du token (opaque ou JWT) et sa durée de vie.
- Le contenu précis encodé dans le QR code.
- Le comportement quand plusieurs télécommandes sont appairées en même temps.
- Le nombre maximal de tentatives du PIN et la durée du verrouillage.
- Ce que concrètement un profil restreint (invité compris) peut ou ne peut pas faire.
