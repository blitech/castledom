# Setup Castledom — notes d’installation

Documente ce qui a été installé / manquait au premier import du projet (2026-10-16).

## Déjà présent

| Outil | Version / chemin |
|-------|------------------|
| Rust / cargo | `1.98.1` (stable MSVC) |
| ripgrep (`rg`) | WinGet + Cursor bundlé |
| RTK | `rtk.exe` (WinGet Links) |

## Installé pour ce projet

| Élément | Commande | Statut |
|---------|----------|--------|
| Cible WASM | `rustup target add wasm32-unknown-unknown` | OK |
| Trunk | `cargo install trunk --locked` → `trunk 0.21.14` | OK |
| Init RTK | `rtk init` → `CLAUDE.md` + `.rtk/filters.toml` | OK |
| wasm-bindgen | Téléchargé automatiquement par Trunk (0.2.129) au 1er `trunk serve` | OK |

## Optionnel / non installé

| Élément | Pourquoi | Commande si besoin |
|---------|----------|--------------------|
| `wasm-server-runner` | Référencé dans `.cargo/config.toml` pour `cargo run` vers WASM sans Trunk | `cargo install wasm-server-runner` |
| Code Memory MCP | Non visible dans les MCP de cette session Cursor | Activer le serveur MCP « code memory » dans les settings Cursor |

## Piège connu : écran noir (WASM / IDE)

Causes vues au premier lancement :

1. **Pas de `main`** — `index.html` doit avoir `data-target-name="castledom"` (binaire).
2. **Panic Bevy B0001** — plusieurs `Query` avec `&mut Sprite` sans `Without<…>`
   (corrigé dans `sync_world_visuals`).
3. **Pas de caméra menu** — l’UI Bevy nécessite un `Camera2d` (`MenuCamera`).
4. **Navigateur IDE** — Simple Browser / preview sans WebGL : ouvrir
   `http://127.0.0.1:8080/` dans Chrome/Edge. Éviter `http://localhost.:8080/`
   (point final bizarre loggé par Trunk).

Vérifier la console : panics, et canvas > 300×150.

## Piège connu : `NO_COLOR=1` + Trunk

Si l’environnement définit `NO_COLOR=1` (fréquent sous Cursor / CI), Trunk 0.21 peut échouer :

`error: invalid value '1' for '--no-color'`

Contournement PowerShell :

```powershell
Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue
trunk serve
```

Ou : `trunk serve --color never`

## Lancer

```bat
REM Tout-en-un (signaling + Trunk) — recommandé en local (CMD)
scripts\dev.bat
scripts\dev.bat -Solo
scripts\dev.bat -Desktop
```

```powershell
# Équivalent PowerShell
.\scripts\dev.ps1
.\scripts\dev.ps1 -Solo
.\scripts\dev.ps1 -Desktop
```

```bash
# Ou séparément
trunk serve                              # → http://127.0.0.1:8080
cargo run -p game_app --bin castledom    # desktop
cargo run -p signaling                   # multi
```

Le script retire `NO_COLOR` et passe `--color never` à Trunk pour éviter le bug Trunk 0.21 sous Cursor.

## Vérifications utiles

```bash
rustup target list --installed   # doit contenir wasm32-unknown-unknown
trunk --version
rtk --help
rg --version
```

## Outils agent (workflow SDD)

Voir `.cursor/skills/castledom-dev/` et `docs/SDD.md`.
Privilégier `rtk` pour les sorties shell (build/test/rg) afin de limiter le bruit tokens.
