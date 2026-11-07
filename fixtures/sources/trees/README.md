# Tree card source art

`iso_pines.png` is from the CC0 "Pixel Pine Tree Assets (10 Pack)" by pistachio:

- https://opengameart.org/content/pixel-pine-tree-assets-10-pack
- License: CC0 1.0 Universal

The track editor uses five green pine sprites packed into a horizontal card atlas.
The atlas adds a baked dark outline and is converted to raw RGBA for the WASM renderer:

```sh
python3 tools/tree-cards/build_atlas.py
```

Outputs:

- `crates/sim-render/assets/tree_cards.png`
- `crates/sim-render/assets/tree_cards.rgba`
