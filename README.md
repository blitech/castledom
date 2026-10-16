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

## Lancer en navigateur

```bash
trunk serve
```

Ouvre `http://127.0.0.1:8080`.

## Desktop (natif)

```bash
cargo run -p game_app --bin castledom
```

## Lobby (solo ou multi)

1. **Solo 1v1 / 2v2** → lobby local, adversaires en IA → *Verrouiller & Lancer*
2. **Créer salon** → code court (ex. `7K4M`) à partager
3. **Rejoindre** → tape le code puis *Rejoindre*
4. Host : mode 1v1/2v2, toggle IA sur slots libres, **Verrouiller & Lancer**

### Multijoueur (signaling)

```bash
cargo run -p signaling
```

Puis ouvre 2+ clients (onglets / appareils) sur le même code.

Salon Matchbox : `ws://127.0.0.1:3536/castledom_<code>`

## Contrôles

| Touche | Action |
|--------|--------|
| 1 / 2 / 3 | Produire Melee / Ranged / Tank |
| WASD / flèches | Caméra |
| Clic droit + glisser | Pan |
| Molette | Zoom |
| Esc | Retour menu |

## Crates

| Crate | Rôle |
|-------|------|
| `game_core` | Simulation déterministe + lobby state |
| `game_render` | Rendu / HUD Bevy 2D |
| `game_net` | Matchbox + lockstep + sync lobby |
| `game_app` | Entrypoint WASM / desktop (`castledom`) |
| `signaling` | Serveur Matchbox full-mesh |
