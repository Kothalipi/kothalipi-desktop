#!/bin/sh
# English test clips from macOS voices, in the layout the accuracy check expects.
set -eu
out=${1:?usage: make-english.sh <folder>}
mkdir -p "$out"
i=0
grep -E '^en[0-9]+	en	' "$(dirname "$0")/sentences.tsv" | while IFS="$(printf '\t')" read -r id lang text; do
  case $i in 0) voice=Daniel ;; 1) voice=Samantha ;; *) voice=Alex ;; esac
  say -v "$voice" -o "$out/$id.aiff" "$text"
  afconvert -f WAVE -d LEI16@16000 -c 1 "$out/$id.aiff" "$out/$id.en.wav"
  rm -f "$out/$id.aiff"
  printf '%s' "$text" > "$out/$id.en.txt"
  i=$((i + 1))
done
echo "wrote English clips to $out"
