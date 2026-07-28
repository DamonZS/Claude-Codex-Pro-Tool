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

## CCP Manager background library

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

Users can also open **Theme Center > CCP Appearance > Add background** without
editing a theme package. CCP validates and stores multiple local high-resolution
images in a content-addressed library, so the same image is not duplicated.
Selecting another card switches immediately. **CCP default background** only
deactivates the current library image; it does not delete saved backgrounds.

## Build your own Codex theme

For most themes, use **CCP Manager > Theme Center > DIY Theme**. The visual
workspace creates and validates the manifest, scoped CSS, preview image, and
optional local background automatically. It exposes glass transmission, blur,
radius, and font scale while deriving tone and an accessible palette from the
selected image. Density stays at the Codex default. Live preview and reopening
a DIY theme for later changes are supported; no JSON or CSS is required.

Use the manual package workflow below only when a theme needs selectors or
layout behavior that the safe visual controls do not expose.

Do not edit files inside CCP's installed theme library. Those files are covered
by integrity records and transaction recovery. Copy one of the directories in
this repository to a new working directory, give it a new ID, and import the
finished directory or ZIP through **Theme Center > Codex Themes > Import theme**.

A complete package intended for sharing normally uses this layout:

```text
my-codex-theme/
  theme.json
  theme.css
  preview.png
  background.png
  LICENSE
  NOTICE.md
```

`theme.json`, its `entry_style` file and the declared preview image are the
technical minimum. Add local artwork only when the theme uses it, and include
the applicable `LICENSE`/`NOTICE.md` whenever the package is distributed.

`theme.json` declares every file that the theme can use. Start with this shape:

```json
{
  "format_version": 1,
  "id": "my-codex-theme",
  "name": "My Codex Theme",
  "version": "1.0.0",
  "author": "Your name",
  "description": "A short description",
  "preview": "preview.png",
  "entry_style": "theme.css",
  "assets": ["theme.css", "background.png"],
  "root_attributes": {
    "classes": ["ccp-theme-my-codex-theme"],
    "attributes": {
      "data-ccp-theme-shell": "dark"
    }
  },
  "asset_variables": {
    "--ccp-theme-art": "background.png",
    "--ccp-theme-manager-background": "background.png"
  }
}
```

Use a globally unique lowercase `id` containing only letters, digits and
hyphens. Root classes must start with `ccp-theme-`; custom root attributes must
start with `data-ccp-theme-`. Keep every CSS selector under your theme class so
the style does not affect Codex when another theme is active:

```css
:root.ccp-theme-my-codex-theme {
  --my-surface: rgb(16 18 24 / 0.82);
}

:root.ccp-theme-my-codex-theme body {
  background: var(--ccp-theme-art) center / cover fixed no-repeat;
}
```

Theme packages are visual assets only. Remote URLs, `@import`, JavaScript,
Renderer injection, unscoped input/menu rewrites, symlinks and files outside
the package are rejected. Use local PNG, JPEG or WebP images. The preview must
be a real screenshot of the theme; a 3840x2160 background is recommended for
high-DPI displays. Keep each file under 8 MB and the complete unpacked package
under 32 MB.

To create a ZIP whose root directly contains `theme.json`:

```powershell
Compress-Archive -Path .\my-codex-theme\* -DestinationPath .\my-codex-theme.zip -Force
```

```bash
cd my-codex-theme && zip -r ../my-codex-theme.zip .
```

Import the directory first while designing, then import the final ZIP to prove
that both forms compile to the same payload. CCP validates the manifest, CSS,
images, paths and size in a temporary directory before atomically installing
the theme. Applying a theme is a separate action and requires restarting Codex.
