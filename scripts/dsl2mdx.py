#!/usr/bin/env python3
"""Convert wikititlepair DSL output to MDict MDX format.

Parses DSL directly (no pyglossary), merges duplicate headwords,
fixes link protocols, and packs with mdict-utils.

Fixes DSL→MDX conversion issues:
  1. Duplicate headwords merged with <br> separators
  2. Headword prepended in bold
  3. bword:// → entry:// with URL-encoded non-ASCII targets

Dependencies:
  pip install mdict-utils

Usage:
  python scripts/dsl2mdx.py wikipedia-titlepair-en-zh-20260603.dsl
  # Output: wikipedia-titlepair-en-zh-20260603.mdx
"""

import argparse
import gzip
import re
import subprocess
import sys
import urllib.parse
from collections import defaultdict
from pathlib import Path


def parse_dsl(path: Path) -> dict[str, list[str]]:
    """Parse DSL file. Returns {headword: [body_line, ...]}."""
    print(f"[1/3] Parsing DSL ({path.name})")
    
    # Handle .dz (dictzip, which is just gzip)
    if path.suffix == '.dz':
        fh = gzip.open(path, 'rt', encoding='utf-8')
    else:
        fh = open(path, 'r', encoding='utf-8')
    
    entries = defaultdict(list)
    headword = None
    count = 0
    
    with fh:
        for line in fh:
            line = line.rstrip('\n\r')
            
            if not line:
                continue
            
            if line.startswith('#') or line.startswith('\t'):
                # Header line or body line
                if headword is not None and (line.startswith('\t') or line.startswith(' ')):
                    body = line.strip()
                    if body:
                        entries[headword].append(body)
                    continue
            else:
                # New headword
                headword = line
                count += 1
                if count % 1000000 == 0:
                    print(f"  Parsed {count:,} entries...")
    
    print(f"  Parsed {count:,} DSL entries, {len(entries):,} unique headwords "
          f"({count - len(entries):,} merged)")
    return entries


def _unescape_dsl(text: str) -> str:
    """Reverse DSL backslash escapes. Does NOT HTML-escape."""
    text = text.replace('\\\\', '\\')
    text = text.replace(r'\(', '(')
    text = text.replace(r'\)', ')')
    text = text.replace(r'\{', '{')
    text = text.replace(r'\}', '}')
    text = text.replace(r'\[', '[')
    text = text.replace(r'\]', ']')
    text = text.replace(r'\#', '#')
    text = text.replace(r'\@', '@')
    text = text.replace(r'\<', '<')
    text = text.replace(r'\>', '>')
    text = text.replace(r'\~', '~')
    text = text.replace(r'\^', '^')
    return text


def _html_escape(text: str) -> str:
    """HTML-escape for safe display in MDX."""
    text = text.replace('&', '&amp;')
    text = text.replace('<', '&lt;')
    text = text.replace('>', '&gt;')
    text = text.replace('"', '&quot;')
    return text


def write_mdx_txt(entries: dict[str, list[str]], txt_path: Path) -> int:
    """Write tab-separated text with link fixes. Returns entry count."""
    print(f"  Writing {txt_path.name}...")
    
    written = 0
    with open(txt_path, 'w', encoding='utf-8') as out:
        for word in sorted(entries.keys(), key=str.lower):
            bodies = entries[word]
            
            # Unescape DSL escapes for HTML display
            word_clean = _unescape_dsl(word)
            word_html = _html_escape(word_clean)
            
            # Merge multiple bodies with <br> separators
            combined = "<br>".join(bodies)
            
            # Convert DSL <<cross-ref>> → HTML <a href="entry://...">
            combined = re.sub(
                r'<<([^>]*)>>',
                lambda m: (
                    f'<a href="entry://{_encode_link_target(_unescape_dsl(m.group(1)))}">'
                    f'{_html_escape(_unescape_dsl(m.group(1)))}</a>'
                ),
                combined,
            )
            
            # Prepend bold headword (HTML-safe)
            definition = f'<b>{word_html}</b><br>{combined}'
            
            # MDX key: unescaped plain text for searchability
            out.write(f'{word_clean}\n{definition}\n</>\n')
            written += 1
    
    print(f"  Wrote {written:,} entries")
    return written


def _encode_link_target(target: str) -> str:
    """URL-encode non-ASCII in link targets."""
    if any(ord(c) > 127 for c in target):
        return urllib.parse.quote(target, safe='')
    return target


def pack_mdx(txt_path: Path, mdx_path: Path, title: str) -> None:
    """Pack tab-separated text into MDX via mdict-utils."""
    print(f"[2/3] Packing MDX ({mdx_path.name})")
    
    title_path = txt_path.parent / '_mdx_title.html'
    desc_path = txt_path.parent / '_mdx_desc.html'
    title_path.write_text(title, encoding='utf-8')
    desc_path.write_text('Wikipedia title pairs from Wikidata.', encoding='utf-8')
    
    subprocess.run([
        'mdict',
        '--title', str(title_path),
        '--description', str(desc_path),
        '-a', str(txt_path),
        str(mdx_path),
    ], check=True)
    
    for p in [title_path, desc_path]:
        p.unlink(missing_ok=True)
    
    print(f"  → {mdx_path} ({mdx_path.stat().st_size / 1e6:.1f} MB)")


def main():
    parser = argparse.ArgumentParser(
        description='Convert wikititlepair DSL to MDict MDX format',
    )
    parser.add_argument('dsl', type=Path, help='Input .dsl or .dsl.dz file')
    parser.add_argument('-o', '--output', type=Path, help='Output .mdx path (default: same name)')
    parser.add_argument('-t', '--title', default='Wikipedia Title Pairs',
                        help='Dictionary title shown in reader')
    parser.add_argument('--keep-txt', action='store_true',
                        help='Keep intermediate tab-separated text file')
    args = parser.parse_args()
    
    dsl_path = args.dsl.resolve()
    if not dsl_path.exists():
        sys.exit(f"Error: {dsl_path} not found")
    
    mdx_path = args.output or dsl_path.with_suffix('.mdx')
    txt_path = mdx_path.with_suffix('.txt')
    
    print(f"DSL → MDX: {dsl_path.name} → {mdx_path.name}\n")
    
    entries = parse_dsl(dsl_path)
    write_mdx_txt(entries, txt_path)
    pack_mdx(txt_path, mdx_path, args.title)
    
    if not args.keep_txt:
        txt_path.unlink()
        print(f"[3/3] Cleaned up {txt_path.name}")
    
    print(f"\nDone. {mdx_path}")


if __name__ == '__main__':
    main()
