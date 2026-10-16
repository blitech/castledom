---
id: SPEC-001
title: Pose DnD bâtiments + grille pixel + combat phys/mag
status: active
crates: [game_core, game_render, game_net, game_app]
created: 2026-10-16
---

# SPEC-001 — Pose DnD bâtiments + grille pixel + combat phys/mag

## Intent

Remplacer la production hotkey par un **placement drag-and-drop** de bâtiments sur la carte. Les bâtiments **produisent automatiquement** des unités. Grille **pixel 32×32**, combat **physique / magique**, HUD ressources (stock + génération / s). Préparer la direction maps tuilées (hors scope).

## Comportement attendu

### HUD ressources

- Affiche le **gold** du joueur local et le **revenu net / s** (ex. `Gold 15 (+8/s)`).
- Départ : **15** gold par faction.
- Revenu = income de base + bonus points de contrôle (existant), exposé clairement à l’UI.

### Pose drag-and-drop

- Barre de construction : icônes + coût.
- **Drag** barre → fantôme snappé grille → **drop** pour poser.
- Fantôme vert / rouge selon validité.
- Annulation : Esc, drop invalide (pas de débit).
- Validité : gold OK, footprint libre, **branching** (`DEPLOY_RADIUS` depuis château ou bâtiment allié).
- Lockstep : `PlaceBuilding { kind, tile_x, tile_y }`.

### Production

- Cooldown puis spawn près du bâtiment ; spawn **gratuit** après pose.
- Hotkeys 1/2/3 production : **retirées**.

### Catalogue v1

Footprints supportés par le moteur : **1×2**, **2×2**, **3×3**. **Tous les bâtiments v1 = 2×2**.

| Bâtiment     | Footprint | Produit  | Rôle                                      | Coût |
|--------------|-----------|----------|-------------------------------------------|------|
| Barracks     | 2×2       | Soldier  | Phys mêlée, mono, armure phys moyenne     | 40   |
| Archery      | 2×2       | Archer   | Phys distance, mono                       | 55   |
| Mage Tower   | 2×2       | Mage     | Magique, petit AoE                        | 70   |
| Temple       | 2×2       | Priest   | Soin allié mono                           | 60   |

Château : 2×2, HQ + ancrage + defense ; ne produit plus.

### Combat

- `DamageType`: Physical | Magical
- `armor_phys` / `armor_mag`, `pen_phys` / `pen_mag`
- `Targeting`: Single | Multitarget { n } | Aoe { radius }
- Priest : `heal_power` → soigne allié blessé (pas d’attaque prioritaire)

Formule :

```
effective_armor = max(0, armor - pen)
damage_taken = damage * (100 / (100 + effective_armor))
```

### Grille / pixel

- `TILE_SIZE = 32` ; unités 1×1 ; bâtiments multi-tuiles ; nearest-neighbor ; sprites procéduraux.

### IA

- Pose bâtiments manquants si gold + slot branching OK (ordre Barracks → Archery → Mage → Temple).

## Hors scope

- Maps mer / terre / arbres / montagnes / cristal (direction future).
- 2e ressource, gatherers.
- Micro / sélection / rally / files manuelles.
- Capacités actives joueur.
- Animations riches, tech tree.

## Design

### `game_core`

- Grille tuiles, buildings, `UnitType` Soldier|Archer|Mage|Priest, combat, `GameInput::place`, gold start 15, `income_per_sec(faction)` helper pour HUD, tests.

### `game_render`

- HUD gold + /s ; build bar DnD ; atlas pixel ; sync bâtiments.

### `game_net` / `game_app`

- Nouveau payload input.

## Vérification

```bash
rtk test -- cargo test -p game_core
rtk err -- cargo check -p game_app --target wasm32-unknown-unknown
# Manuel : DnD 4 bâtiments, HUD gold/s, branching, Priest heal, Mage AoE
```

## Décisions actées

- DnD dès v1 ; branching oui
- 32×32 tuile ; footprints moteur 1×2 / 2×2 / 3×3 ; **v1 = 2×2**
- Soldier, Archer, Mage, **Priest**
- Formule armure % validée
- Gold départ **15** + affichage génération / s
- Maps riches : futur

## Notes / écarts

Implémenté 2026-10-16 :
- DnD barre bas, ghost vert/rouge, branching, gold 15 + HUD `/s`
- 4 bâtiments 2×2, unités Soldier/Archer/Mage/Priest, combat phys/mag + heal
- Sprites procéduraux 32px (unités) / 64px (bâtiments 2×2)
- Hotkeys production retirées
- À tester manuellement via `trunk serve`

### Bugfix — écran noir WASM (B0001)

`sync_world_visuals` avait plusieurs `Query<&mut Sprite>` non disjointes → panic Bevy B0001 au boot. Corrigé avec `Without<…>` sur units/buildings/castles/points.
