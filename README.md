# Castledom

RTS hybride **Castle Wars × Domination Wars** en Rust / Bevy — cible **Web (WASM)** d’abord.

- N camps (`FactionId`) + alliances (`TeamId`) → **1v1** et **2v2**
- Points de contrôle, production continue, flow fields
- Lobby type **Cémantix** : code salon, slots Humain/IA, host verrouille & lance
- Solo vs IA via le même lobby (slots en IA, sans signaling)
- Multijoueur navigateur : Matchbox / WebRTC + lockstep

## Prérequis

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk   # si besoin
```

Notes d’install détaillées (ce qui manquait au premier import) : [docs/SETUP.md](docs/SETUP.md).  
Méthodologie Spec-Driven Development : [docs/SDD.md](docs/SDD.md) — specs dans `specs/`.

## Lancer (tout-en-un)

```bat
scripts\dev.bat
```

Démarre le **signaling** Matchbox + **Trunk** (WASM). Ouvre `http://127.0.0.1:8080`.  
(Équivalent PowerShell : `.\scripts\dev.ps1`.)

| Variante | CMD / bat | PowerShell |
|----------|-----------|------------|
| Solo (sans signaling) | `scripts\dev.bat -Solo` | `.\scripts\dev.ps1 -Solo` |
| Desktop natif + signaling | `scripts\dev.bat -Desktop` | `.\scripts\dev.ps1 -Desktop` |

### Commandes séparées

```bash
trunk serve                              # web seul → http://127.0.0.1:8080
cargo run -p game_app --bin castledom    # desktop natif
cargo run -p signaling                   # multi → ws://127.0.0.1:3536/castledom_<code>
```

## Lobby (solo ou multi)

1. **Solo 1v1 / 2v2** → lobby local, adversaires en IA → *Verrouiller & Lancer*
2. **Créer salon** → code court (ex. `7K4M`) à partager
3. **Rejoindre** → tape le code puis *Rejoindre*
4. Host : mode 1v1/2v2, toggle IA sur slots libres, **Verrouiller & Lancer**

Multijoueur : avec `.\scripts\dev.ps1` (ou `cargo run -p signaling` à part), ouvre 2+ clients sur le même code.
## Contrôles

| Action | Détail |
|--------|--------|
| Drag & drop barre bas | Poser Caserne / Archerie / Tour de mage / Temple |
| HUD haut-gauche | Gold + revenu / s |
| WASD / flèches | Caméra |
| Clic droit + glisser | Pan |
| Molette | Zoom |
| Esc | Annule pose en cours, sinon retour menu |

## Crates

| Crate | Rôle |
|-------|------|
| `game_core` | Simulation déterministe + lobby state |
| `game_render` | Rendu / HUD Bevy 2D |
| `game_net` | Matchbox + lockstep + sync lobby |
| `game_app` | Entrypoint WASM / desktop (`castledom`) |
| `signaling` | Serveur Matchbox full-mesh |
