# Spine

[![CI](https://github.com/Valou-31/Spine/actions/workflows/ci.yml/badge.svg)](https://github.com/Valou-31/Spine/actions/workflows/ci.yml)

Petite app macOS qui surveille un dossier de séries/mangas et fait remonter le numéro d'épisode ou de tome — souvent caché en fin de nom de fichier — dans le commentaire Finder, sans renommer les fichiers ni élargir la colonne Nom.

## Comment ça marche

- Surveille un dossier (et ses sous-dossiers) en tâche de fond.
- Détecte des patterns par regex dans le nom des fichiers (`S01E05`, `T01`, `Vol.05`, `Volume 6`...).
- Écrit le résultat dans le commentaire Finder du fichier, via Finder lui-même (Apple Events) — c'est la seule méthode qui fonctionne réellement, écrire l'attribut étendu directement ne suffit pas.
- Active la colonne **Commentaires** dans Finder (Présentation > Options de présentation) pour voir le résultat.

## Build & lancement

```
cargo build --release
./build_app.sh          # génère dist/Spine.app (avec icône)
open "dist/Spine.app"
```

Au premier clic sur "Démarrer", macOS demande l'autorisation de contrôler Finder (Automation) — à accepter.

## Utilisation

Dans l'interface : dossier à surveiller, liste de patterns (regex, activables/désactivables), extensions de fichiers surveillées, et option d'écraser ou non un commentaire existant. Tout est sauvegardé automatiquement.

Patterns par défaut :

| Nom | Exemple détecté |
|---|---|
| Episode (SxxExx) | `S01E05` |
| Tome/Volume | `T01`, `Vol.05`, `Volume 6` |

## Configuration

Sauvegardée dans `~/Library/Application Support/Spine/config.json`.
