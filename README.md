# Roulette Sport — v0.27

Appli Windows (Rust + egui) qui tire 3 exercices par séance, façon loto.

## Lancer

1. Installer Rust : https://rustup.rs (choisir l'option par défaut, MSVC).
2. Dans ce dossier :
   ```
   cargo run --release
   ```
   L'exécutable est ensuite dans `target\release\roulette-sport.exe`.

## macOS

Chaque envoi sur `main` compile automatiquement l'app sur GitHub (onglet **Actions** › dernière exécution « Compilation » › **Artifacts**) :

- `RouletteSport-macOS` : `Roulette Sport.app` universelle (Apple Silicon + Intel, macOS 11+).
- `RouletteSport-Windows` : `roulette-sport.exe`.

L'app n'est pas signée par Apple : au premier lancement, clic droit sur l'app › **Ouvrir** › **Ouvrir**. Si macOS dit qu'elle est « endommagée », dans le Terminal : `xattr -dr com.apple.quarantine "/Applications/Roulette Sport.app"`.

Sur Mac, les données sont dans `~/Library/Application Support/RouletteSport/` et les polices perso dans le sous-dossier `polices` (ou un dossier `polices` posé à côté de l'app).

## Règles implémentées

- **Navigation verticale (option)** : Réglages › Développement · Dimensions › Navigation « Verticale à gauche » : fine colonne à gauche avec le logo en haut, les onglets en pilules verticales (texte tourné de 90°) et le bouton Réglages en bas. La page se décale d'autant.
- **Navigation** : en-tête flottant au-dessus de la page — faux logo (lettre dans une capsule) à gauche, « AUJOURD'HUI » et « RÉSUMÉ ET ORGANISATION » en deux pilules centrées (le fond de la pilule active glisse de l'une à l'autre), Réglages en bouton rond à droite (engrenage). La page occupe toute la fenêtre et défile sous l'en-tête, qui s'estompe vers le bas ; le contenu commence sous l'en-tête pour ne jamais se chevaucher. La date du jour est sous le titre de la page principale.

- **Filtre VHS** : bruit animé façon cassette sur toute la fenêtre — grain qui scintille, lignes de balayage, bande de tracking qui descend, parasites horizontaux. Dessiné au-dessus de tout sans gêner les clics. Réglable dans Réglages › Développement · Filtre VHS (activer, intensité de chaque effet, vitesse ; 0 = figé).
- **Filtre bitmap** : vrai post-traitement de l'image finale (shader OpenGL) — gros pixels, tramage ordonné (Bayer) et palette réduite. Réglages › Développement · Filtre bitmap : activer, couleurs (palette fond/texte du thème, niveaux de gris, couleurs réduites), niveaux (2 = 1 bit), taille des pixels, tramage, intensité. Appliqué après le filtre VHS.
- **Panneaux des roulettes** (réglages séparés, Réglages › Développement · Panneaux des roulettes) : forme (rayon des coins, bordure pendant le tirage et à l'arrêt, lignes guides), verre (teinte + dosage, opacité du fond, dégradé vertical, reflet brillant + hauteur, ombre de profondeur façon tambour + taille, dessinée par-dessus les noms) et leur propre liseré lumineux.
- **Boutons « À FAIRE » / « FAIT »** (Réglages › Développement · Boutons) : hauteur (part de la roulette), écart avec la roulette, rayon des coins, bordures à faire / fait, taille du texte, opacité, dégradé vertical, reflet brillant + hauteur, liseré lumineux propre.
- **Interrupteurs d'effets** (Réglages › Développement · Effets) : une case par effet ajouté (dégradé de fond, VHS, bitmap ; roulettes : teinte, dégradé, reflet, profondeur, liseré ; boutons : dégradé, reflet, liseré ; liseré des autres panneaux) + « Tout désactiver (couleur unie) » / « Tout réactiver ». Les mêmes cases sont aussi devant chaque réglage. Décocher garde les valeurs des curseurs.
- **Aperçu des réglages** : dans l'onglet Réglages, un aperçu fixe à droite montre en direct une roulette, les boutons « à faire » / « fait » et un panneau avec les réglages en cours (bouton « masquer » / « Aperçu »).
- **Autres panneaux** : aplats avec un liseré lumineux façon « liquid glass » (Réglages › Développement · Liseré des autres panneaux : activer, intensité, direction de la lumière, largeur, halo, reflet opposé, concentration, couleur).
- **Titre de la page principale** : centré, 3× plus grand (réduit seulement au-delà de 2 lignes), il change selon l'avancement : « Pas encore commencé ? » → « Il s'agirait de s'y mettre » → « Encore 2, qu'est-ce que tu peux être long » → « Plus qu'un, on voit le bout » → « Ah bah enfin » (fondu à chaque changement).
- **Roulettes** : parfaitement carrées (côté max 320 px), centrées horizontalement et verticalement dans la fenêtre ; le grand titre est collé juste au-dessus (il se réduit si besoin) ; le bouton du bas a la même largeur. Fenêtre de 1040 × 940 au lancement.
- **Mise en page centrée** : Réglages et Résumé et organisation sont des colonnes centrées, contenus centrés.
- **Tirage** : 1 tirage par jour de sport, 3 exos (réglable). Chaque case est une roue qui défile du haut vers le bas ; à l'arrêt la case vibre et sa bordure s'épaissit.
- **Bouton fait** : sous chaque roue, même largeur, 1/3 de sa hauteur. Gris-rouge « À FAIRE », vert « FAIT », avec la même vibration que l'arrêt de la roue.
- **Saisie après « À FAIRE »** : la case double de largeur et s'allonge, puis pose 3 questions, texte centré : nombre de séries → reps de chaque série → poids de chaque série (pas de 0,5 kg, ou bouton « Poids du corps »). Chaque valeur se choisit sur une roue de nombres (molette : 1 cran = 1 valeur, glisser avec inertie, clic sur une valeur, flèches ↑/↓), avec un dégradé d'opacité vers le haut et le bas. Boutons empilés : SUIVANT / VALIDER au-dessus, RETOUR dessous (Entrée / Échap). Valeurs pré-remplies avec la dernière séance de l'exercice. À la validation, la case reprend sa taille et passe en « FAIT ».
- **Séance terminée** (les 3 cochés) : l'écran vibre, le fond s'assombrit, des confettis tombent et un message donne la série de jours complétés d'affilée (repos et weekends ne cassent pas la série, un jour de sport non terminé si). Clic, Entrée ou Échap pour fermer.
- **Journée terminée** : les cases ne peuvent plus être décochées. Toute la page passe sous un voile : contenu flouté, vignette plus sombre sur les bords qu'au centre. « Reviens demain champion, c'est bon pour aujourd'hui. » s'affiche au centre, en grand et épais ; le bouton « SÉANCE TERMINÉE » reste net au-dessus. Les onglets restent accessibles.
- **Bouton Lancer / Relancer** : fixé en bas de la page, largeur des trois roues, en capsule comme les onglets de l'en-tête (fond plein comme l'onglet actif ; grisé quand indisponible). « SÉANCE TERMINÉE » prend la même forme.
- **Onglet Résumé et organisation** : progression de la semaine, calendrier mensuel (accomplie / loupée / repos / prévue / weekend, « x/3 » faits dans chaque case) et base d'exercices. Le détail du jour sélectionné est à côté du calendrier (exercices, fait / non fait, séries · reps · poids, relances). Les journées archivées par un reset ne sont plus affichées. L'ancien onglet Historique est intégré ici.
- **Base d'exercices** (vue galerie, lecture seule) : une carte par exercice avec 3 chiffres mis en avant — reps max (meilleure série), poids max, répétitions totales — calculés à partir des séances saisies. Clic sur une carte : elle double de taille et affiche séances, séries totales, dernière fois et les 5 dernières séances. Recherche par nom. Carte « + » en fin de galerie pour ajouter un exercice ; clic droit sur une carte pour le renommer ou le supprimer (avec validation).
- **Animations** : toutes en ease-in-out (départ et arrivée en douceur) — pilule d'onglet qui glisse, engrenage qui tourne, fondu + léger glissement du contenu au changement d'onglet, agrandissement des cases et des cartes, couleur du bouton fait, apparition de la célébration. Les vibrations restent des secousses amorties.
- **Reset du calendrier** (Réglages › Calendrier) : avec fenêtre de validation. Toutes les journées existantes (y compris aujourd'hui et d'éventuels jours simulés) sont archivées : le calendrier, la série et la semaine en cours repartent de zéro, et on peut retirer aujourd'hui. Les séances enregistrées restent dans la base d'exercices et dans l'historique.
- **Relances** : 2 par jour (réglable). Une relance évite les exos qu'on vient de tirer.
- **Liste hebdo** : les exos pas encore tombés cette semaine sont prioritaires. La liste repart à zéro chaque lundi.
- **Planification** (Réglages) : soit un cycle N jours de séance / M jours de repos (weekends sans séance), soit « Modèle par semaine » : on clique sur chaque jour (lun–dim) d'une semaine type pour choisir séance ou repos. Aperçu des 14 prochains jours dans les deux cas.
- **Alerte** si la liste restante dépasse les places disponibles avant le weekend.
- **Historique** : chaque tirage est enregistré (exos, relances utilisées, exos cochés "fait", détail des séries) et consultable depuis le calendrier.
- **Développement (Réglages, temporaire)** : police par catégorie (titres, texte courant, chiffres, roulettes — polices intégrées + polices Windows trouvées) palette de couleurs (10 teintes éditables + préréglages Perso (palette claire par défaut), Monochrome, Ardoise, Sable, Nuit violette, Clair) dégradé de fond (activable, couleur d'arrivée, sens vertical / horizontal / diagonale / radial, intensité) et dimensions (hauteur de l'en-tête, épaisseur des bordures des panneaux). Les choix sont enregistrés.

## Polices perso

Les polices perso ne sont pas incluses dans le dépôt (licences « personal use ») : sans elles, l'app utilise sa police intégrée.

Dépose des fichiers `.ttf` / `.otf` / `.ttc` dans `%APPDATA%\RouletteSport\polices` ou dans le dossier `polices` du projet, puis Réglages › Polices › « Actualiser la liste ». Elles apparaissent avec la mention « (perso) ». Le bouton « Ouvrir le dossier » ouvre le dossier dans l'Explorateur.

## Réglages par défaut

Les réglages par défaut (nouvelle installation, boutons « par défaut ») reprennent la configuration du 20/10/2026 : modèle par semaine lun–ven, 10 relances, palette Perso (sombre ; l'ancienne claire reste en « Perso clair »), polices MADE TOMMY (perso, police intégrée si absentes), en-tête 89 px, bordures 0,75 px, VHS et bitmap désactivés, roulettes à coins droits avec profondeur 25 %, liseré des autres panneaux à 8 %.

## Données

`%APPDATA%\RouletteSport\data.json` (modifiable à la main si besoin).

## Code

- `src/theme.rs` — palette, polices, dégradé de fond, dimensions et fond des panneaux + tests
- `src/bitmap.rs` — filtre bitmap (shader de post-traitement)
- `src/vhs.rs` — filtre VHS (grain, balayage, tracking, parasites) + tests
- `src/veil.rs` — voile de fin de séance (flou, vignette)
- `src/wheel.rs` — roue de sélection de nombre
- `src/anim.rs` — courbe ease-in-out commune et valeurs animées + test
- `src/database.rs` — base d'exercices en galerie
- `src/celebrate.rs` — vibration, confettis, message de félicitations
- `src/reel.rs` — cinématique des roues (accélération, vitesse plafonnée anti-effet stroboscopique, arrêt pile sur le nom) + tests
- `src/model.rs` — règles (tirage, repos, reset hebdo) + tests (`cargo test`)
- `src/app.rs` — interface monochrome (roues : `ui_slots`, saisie : `ui_wizard`, validations : `ui_modal`)
- Onglet Réglages > Test : décaler la date pour simuler une semaine.
