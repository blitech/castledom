---
name: castledom-dev
description: >-
  Agent de développement Castledom (Rust/Bevy WASM RTS) en Spec-Driven Development.
  Utilise rtk, ripgrep et Code Memory MCP. À lire pour toute feature, bug, refactor,
  ou quand l’utilisateur mentionne SDD, specs, castledom, lobby, lockstep, Matchbox.
---

# Castledom Dev (SDD)

## Quand s’activer

Feature, bug non trivial, refactor cross-crate, net/lobby/sim, ou demande explicite SDD / agent castledom.

## Workflow obligatoire

```
Task Progress:
- [ ] 1. Contexte (memory + rg)
- [ ] 2. Spec (créer/MAJ)
- [ ] 3. Review intent avec user si ambigu
- [ ] 4. Implement (scope = spec)
- [ ] 5. Verify (commandes de la spec)
- [ ] 6. Close spec + memory
```

### 1. Contexte

1. Si **Code Memory MCP** est dispo : requête sur le sujet (crates, décisions, patterns).
2. Sinon / en complément :
   ```bash
   rtk rg "motif" -g "*.rs"
   rtk tree -L 3
   ```
3. Lire `docs/SDD.md`, `docs/SETUP.md`, `README.md` si besoin d’orientation.

### 2. Spec

- Nouveau travail → `specs/active/SPEC-NNN-slug.md` depuis `specs/_template.md`
- Numéro : max existant + 1 (chercher `specs/**/SPEC-*.md`)
- Remplir : intent, comportement, hors scope, crates, vérif
- `status: active` avant le code

### 3. Review

Si l’intent ou le hors-scope est flou → **demander** avant d’écrire du code.

### 4. Implement

- Respecter les frontières de crates (`game_core` déterministe ; rendu dans `game_render` ; net dans `game_net`)
- Pas de dépendances inutiles ; style existant
- Pas de docs markdown hors `specs/` / demande user (sauf MAJ SETUP si nouvel outil)

### 5. Verify

Préférer RTK pour limiter les tokens :

```bash
rtk test -- cargo test -p <crate>
rtk err -- cargo check -p <crate>
# Web
trunk serve   # http://127.0.0.1:8080
```

### 6. Close

- Spec → `status: done`, notes d’écarts, déplacer vers `specs/done/` si pertinent
- Code Memory : enregistrer décision d’archi / piège trouvé (si MCP dispo)

## Outils

| Outil | Usage |
|-------|--------|
| `rtk rg` / `rg` | Recherche code |
| `rtk test` / `rtk err` / `rtk summary` | Build & tests condensés |
| `rtk tree` / `rtk ls` | Structure |
| Code Memory MCP | Mémoire projet durable |
| Trunk | Cible web principale |

## Anti-patterns

- Coder sans spec pour une feature gameplay/net
- Relancer des builds verbeux sans `rtk` quand RTK est dispo
- Toucher `game_core` déterminisme sans tests / smoke
- Ignorer le hors-scope de la spec
