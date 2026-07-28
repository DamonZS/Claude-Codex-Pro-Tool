# CCP Theme Packages

These are local CCP theme packages adapted from the visual assets in:

`H:\xunlei\Codex-Dream-Skin-main\Codex-Dream-Skin-main`

Packages:

- `codex-dream-skin-macos`: macOS stylesheet and preview.
- `codex-dream-skin-windows`: Windows stylesheet and preview.

Each package is a directory package accepted by the CCP theme importer. The
package contains a CCP `theme.json`, one CSS entry file, and one preview image.
The original Dream Skin `renderer-inject.js` files are intentionally excluded:
CCP theme packages are visual CSS assets, while renderer injection remains an
independent runtime capability.

The source project is distributed under the MIT license. The license file is
included in each package.

## GitHub curated themes

The following optional themes are adapted from
[Theme Studio for Codex](https://github.com/ericsi-lab/codex-theme-studio) at
commit `9ff093338de907d6120e3ce6c7915ffd55f98e1f`:

| Theme ID | Display name | Directory | ZIP |
| --- | --- | --- | --- |
| `aurora-glass` | 极光穹顶 | `aurora-glass/` | `aurora-glass.zip` |
| `clockwork-fox-spirit` | 机关狐灵 | `clockwork-fox-spirit/` | `clockwork-fox-spirit.zip` |
| `cyber-changan` | 赛博长安 | `cyber-changan/` | `cyber-changan.zip` |
| `obsidian-gold` | 黑金环域 | `obsidian-gold/` | `obsidian-gold.zip` |
| `verdant-sanctuary` | 森光秘境 | `verdant-sanctuary/` | `verdant-sanctuary.zip` |
| `lotus-fire-nezha` | 莲火哪吒 | `lotus-fire-nezha/` | `lotus-fire-nezha.zip` |

Upstream code is MIT licensed. The included background artwork is licensed
under CC BY 4.0 with attribution to Theme Studio for Codex contributors. The
preview files are format-only PNG conversions of the corresponding upstream
`docs/examples/real/<theme-id>/new-task.webp` screenshots. Product UI visible
in those screenshots remains subject to its respective owner's rights.

The CCP packages contain only a manifest, scoped CSS, local PNG assets,
license, and notice. Upstream Renderer injection, CDP control, installers, and
remote loading code are intentionally excluded. Importing a package does not
apply it automatically or change Provider, model, credential, localization,
input, menu, or session settings.

## Optional Manager background

A theme package can also provide the environment background used behind the
CCP Manager liquid-glass shell. Add the local image to `assets`, then map it in
`asset_variables`:

```json
{
  "assets": [
    "theme.css",
    "manager-background.webp"
  ],
  "asset_variables": {
    "--ccp-theme-manager-background": "manager-background.webp"
  }
}
```

`--ccp-theme-manager-background` has priority. Existing packages that only
declare `--ccp-theme-art` remain compatible and use that image for both Codex
and the Manager. The Manager consumes only the validated local image data URI;
it does not execute the package's Codex Renderer CSS, classes, or attributes.

Use an image of at least 1920x1080; 3840x2160 WebP or JPEG remains recommended
for high-DPI displays. PNG, JPEG, and WebP are accepted, but each file must
remain under 8 MB and the complete package under 32 MB. Keep important content
away from the edges because the Manager uses a centered `cover` crop at
different window sizes. If the variable is absent or invalid, the built-in
light or dark Manager background is used automatically.

Users can also choose **Theme Center > Set background** without editing a
theme package. That local override has the highest priority and is stored in
CCP's theme state directory; **Restore theme background** removes only the
override and falls back to the active theme variable or the built-in image.
