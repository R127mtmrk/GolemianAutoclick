# GolemianAutoclick
*(Project by R127)*

## Description
**GolemianAutoclick** est une application Rust d’**auto-clic** (auto-clicker) destinée à automatiser des clics de souris selon une configuration simple (démarrage/arrêt, cadence, éventuellement un mode “maintenir” ou “toggle” selon l’implémentation).

> Objectif : fournir un petit outil léger, rapide et portable (via Rust) pour automatiser des actions répétitives.

## Fonctionnalités (général)
- Lancement d’une boucle de clics à interval régulier  
- Contrôle du démarrage / arrêt (par raccourci clavier et/ou via l’interface console, selon le projet)
- Paramétrage de la cadence (clics par seconde / délai en ms)
- Exécutable compilé via `cargo`

> Le logiciel est mis à jour régulièrement, n'hésite pas à réinstaller si tu vois que ta version à trop de bug.

## Prérequis
- Environnement Windows (possibilité d'éxécuter le programme en administrateur)

## Installation
- Rejoingnez le Discord du projet : https://discord.gg/txJF5CbFtH

## Utilisation
Selon la manière dont l’application est conçue, l’usage se fait typiquement :
1. Lancer l’application en Administrateur (une pop-up d'erreur s'affichera si ce n'est pas le cas)
2. Définir la cadence de clic (1-100 clics par seconde)
3. Utiliser la commande / touche prévue pour **démarrer** l’auto-clic
4. Utiliser la commande / touche prévue pour **arrêter** l’auto-clic

### Exemple de workflow (à adapter)
- **Start** : une touche dédiée (ex: `F4`)
- **Stop** : une touche dédiée (ex: `F4 / E`)
- **Quit** : `Esc` ou `Ctrl+C`

## Structure du projet
- `src/main.rs` : point d’entrée de l’application
- `Cargo.toml` : configuration Cargo + dépendances
- `README.md` : documentation

## Sécurité & responsabilité
Un auto-clicker peut être considéré comme une **triche** dans certains jeux/logiciels ou violer des **conditions d’utilisation**.  
Utilise cet outil **à tes risques** et uniquement dans un cadre autorisé (tests, accessibilité, automatisation personnelle, etc... ).

## Evolution
Le Discord joint dispose d'un channel `✏️・avis-logiciel-et-idée`, permettant de transmettre les bugs trouvés et des idées à intégrer dans le projet.