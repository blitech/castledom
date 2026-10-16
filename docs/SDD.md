# Spec-Driven Development (SDD) — Castledom

Toute feature non triviale passe par une **spec** avant le code.

## Cycle

1. **Intent** — besoin utilisateur en 1–3 phrases
2. **Spec** — fichier dans `specs/` (template ci-dessous)
3. **Review** — valider la spec (humain ou agent) avant implémentation
4. **Implement** — code aligné sur la spec ; pas de scope creep
5. **Verify** — tests / smoke listés dans la spec
6. **Close** — marquer la spec `status: done`, noter les écarts

## Emplacement

```
specs/
  README.md
  _template.md
  active/          # en cours
  done/            # terminées (archive légère)
```

## Règles

- Pas de feature « au feeling » sans spec si ça touche gameplay, net, ou API publique des crates.
- Bugs hotfixes : mini-spec inline OK (section *Bugfix* du template) si < ~30 min.
- La spec est la source de vérité ; si le code diverge, mettre à jour la spec **ou** corriger le code.
- Un changement = une spec (ou une section claire si extension d’une spec active).

## Crates & frontières

| Crate | Responsabilité |
|-------|----------------|
| `game_core` | Sim déterministe, lobby, IA, flow fields |
| `game_render` | Rendu / HUD Bevy 2D |
| `game_net` | Matchbox, lockstep, sync lobby |
| `game_app` | Entrypoint WASM / desktop + UI lobby |
| `signaling` | Serveur Matchbox |

Les specs doivent indiquer **quelles crates** bougent.
