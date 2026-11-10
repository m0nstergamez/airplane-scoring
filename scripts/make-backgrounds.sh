#!/usr/bin/env bash
# Render the level backgrounds: art/levels/levelN/background.svg
# -> web/levels/levelN.png, then check that each image tiles without a seam
# (the game repeats it side by side while it scrolls).
#
# Needs rsvg-convert and ImageMagick (macOS: brew install librsvg imagemagick).
# Run it after changing an SVG and commit the PNGs, like make-sounds.py.
#   scripts/make-backgrounds.sh            all levels
#   scripts/make-backgrounds.sh level1     one level
set -euo pipefail
cd "$(dirname "$0")/.."

# Rendered at twice the SVG size so it stays sharp on phone screens
# (the game scales it to the screen height).
SCALE=2
# Seam check: a pixel "jumps" when it differs from its neighbour across
# the seam by more than JUMP percent. Shape edges that cross the seam at a
# slant make a few such pixels anyway, so up to MAX_SEAM percent of the
# seam may jump. A real seam (one band of sky in a different colour) is
# far above that.
JUMP=3
MAX_SEAM=1

for tool in rsvg-convert magick; do
  command -v "$tool" >/dev/null || { echo "missing: $tool (brew install librsvg imagemagick)" >&2; exit 1; }
done

mkdir -p web/levels
levels=("$@")
if [ ${#levels[@]} -eq 0 ]; then
  for d in art/levels/*/; do levels+=("$(basename "$d")"); done
fi

for level in "${levels[@]}"; do
  svg="art/levels/$level/background.svg"
  png="web/levels/$level.png"
  [ -f "$svg" ] || { echo "missing: $svg" >&2; exit 1; }

  width=$(sed -n 's/.*viewBox="0 0 \([0-9]*\) [0-9]*".*/\1/p' "$svg" | head -1)
  rsvg-convert -w $((width * SCALE)) "$svg" -o "$png.tmp"
  # Drop metadata (timestamps) so re-rendering the same SVG gives the same file.
  magick "$png.tmp" -strip "PNG24:$png"
  rm "$png.tmp"

  # Seam: the last column of one copy sits next to the first of the next.
  w=$(magick identify -format '%w' "$png")
  seam=$(magick "$png" \( -clone 0 -crop "1x+0+0" +repage \) \( -clone 0 -crop "1x+$((w - 1))+0" +repage \) \
    -delete 0 -compose difference -composite -colorspace gray -threshold "$JUMP%" -format '%[fx:mean*100]' info:)
  printf '%s: %s (%s bytes), seam jumps at %.2f %% of its height\n' "$level" "$png" "$(wc -c < "$png" | tr -d ' ')" "$seam"
  if awk "BEGIN{exit !($seam > $MAX_SEAM)}"; then
    echo "  seam too visible: left and right edges of $svg do not match" >&2
    exit 1
  fi
done
