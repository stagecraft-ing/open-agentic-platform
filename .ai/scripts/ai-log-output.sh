#!/usr/bin/env sh
# Usage: ./.ai/scripts/ai-log-output.sh <tool> <output-file>
# Appends a one-line entry under "## Recent outputs" in current.md (deduped).
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/.ai/handoff/current.md"

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <tool> <output-file>" >&2
  exit 1
fi

TOOL="$1"
OUT="$2"

if [ ! -f "$FILE" ]; then
  echo "error: $FILE not found" >&2
  exit 1
fi

DATE=$(date -u +"%Y-%m-%d" 2>/dev/null || date -u +"%Y-%m-%d")

perl -e '
  use strict;
  use warnings;
  my ($path, $date, $tool, $out) = @ARGV;
  open my $fh, "<", $path or die "$path: $!";
  local $/;
  my $t = <$fh>;
  close $fh;

  my $want = "- $date — $tool — $out";
  my $add  = "$want\n";

  my $marker = "\n## Recent outputs\n";
  if (index($t, $marker) < 0) {
    $t =~ s/\s*$/\n\n## Recent outputs\n\n$add/;
  } else {
    my $i = index($t, $marker);
    my $head = substr($t, 0, $i + length($marker));
    my $tail = substr($t, $i + length($marker));
    my $j = index($tail, "\n## ");
    my $body  = ($j < 0) ? $tail : substr($tail, 0, $j);
    my $after = ($j < 0) ? "" : substr($tail, $j);
    my @bullets = grep /^- /, split /\n/, $body;
    exit 0 if @bullets && $bullets[-1] eq $want;
    $body =~ s/\s*$/\n/ if $body ne "" && $body !~ /\n\z/;
    $t = $head . $body . $add . $after;
  }

  open my $w, ">", $path or die "$path: $!";
  print $w $t;
  close $w;
' "$FILE" "$DATE" "$TOOL" "$OUT"

echo "Logged output in $FILE: - $DATE — $TOOL — $OUT"
