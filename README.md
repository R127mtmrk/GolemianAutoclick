# GolemianAutoclick

Projet auto-clicker sous **Tauri** avec:
- backend Rust dans `src-tauri/`
- frontend Vite moderne dans `src/`
- hotkeys globales rebindables (toutes touches clavier)
- correction du bug de mouvement involontaire de la souris
- correction du lancement qui pouvait ouvrir une fenêtre cassée sur `localhost`

## Stack
- Tauri v1
- Rust
- Vite + JavaScript

## Structure
- `src-tauri/src/main.rs`: logique autoclick + commandes Tauri + hotkeys globales
- `src-tauri/tauri.conf.json`: config Tauri
- `src/main.js`: logique UI (`invoke` / événements)
- `src/styles.css`: interface moderne
- `index.html`: shell frontend principal

## Ce qui a été corrigé
- Le clic natif Windows utilise uniquement `LEFTDOWN` + `LEFTUP`, sans événement `MOVE`, ce qui évite le drift de la souris.
- Les deux hotkeys sont modifiables à chaud depuis l'interface.
- Toutes les touches clavier captées par `rdev` peuvent être assignées.
- Les doublons entre raccourcis sont refusés.
- `Échap` annule un rebinding en cours.
- L'application ne se relance plus automatiquement en administrateur au démarrage: c'était une cause probable du bug de fenêtre affichant une page `localhost` inaccessible en mode dev/relaunch.
- L'interface affiche maintenant l'état admin, le statut runtime et les messages backend.

## Développement
```powershell
npm install
npm run tauri:dev
```

## Build
```powershell
npm install
npm run tauri:build
```

## Packaging Windows
Le projet compile et génère l'application Tauri, mais le **bundle installable Windows** dépend d'outils externes comme **NSIS** ou **WiX Toolset**. Dans cet état du dépôt, le build Tauri applicatif passe; si tu veux un `.msi` ou `.exe` installable signé/bundlé, il faudra installer l'outillage Windows correspondant.
