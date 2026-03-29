#!/usr/bin/env sh
# Usage: ./.ai/scripts/ai-claim-baton.sh <current-owner> <next-owner>
# Updates Baton section key lines in .ai/handoff/current.md
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/.ai/handoff/current.md"

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <current-owner> <next-owner>" >&2
  exit 1
fi

CUR="$1"
NXT="$2"

if [ ! -f "$FILE" ]; then
  echo "error: $FILE not found" >&2
  exit 1
fi

TS=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u +"%Y-%m-%dT%H:%M:%SZ")

perl -e '
  use strict;
  use warnings;
  my $path = shift @ARGV;
  my ($cur, $nxt, $ts) = @ARGV;
  open my $fh, "<", $path or die "$path: $!";
  local $/;
  my $text = <$fh>;
  close $fh;
  $text =~ s/^- Current owner: .*/- Current owner: $cur/m;
  $text =~ s/^- Next owner: .*/- Next owner: $nxt/m;
  my $line = "- Last baton update: $ts — baton claimed ($cur -> $nxt)\n";
  if ($text =~ /^- Last baton update: /m) {
    $text =~ s/^- Last baton update: .*$/$line/m;
  }
  elsif ($text =~ /^(- Next owner:.*\n)/m) {
    $text =~ s/^(- Next owner:.*\n)/$1$line\n/m;
  }
  else {
    die "Could not find - Next owner: or - Current owner: anchors in $path\n";
  }
  open my $out, ">", $path or die "$path: $!";
  print $out $text;
  close $out;
' "$FILE" "$CUR" "$NXT" "$TS"

echo "Updated baton in $FILE"
echo "  Current owner: $CUR"
echo "  Next owner:    $NXT"
echo "  Time (UTC):    $TS"
