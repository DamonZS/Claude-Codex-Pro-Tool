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
