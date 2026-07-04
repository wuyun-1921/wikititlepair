# wikitools

CLI tools for building dictionaries from Wikimedia data.

## Features

- `wikitools pair` — Generate bidirectional DSL dictionaries from Wikipedia interlanguage links via Wikidata
- `wikitools titles` — Extract all article titles from a Wikimedia project and build an MDX dictionary linking each title to its online page

## Usage

```bash
# Build a bidirectional EN↔ZH dictionary, can be any two languages
wikitools pair en zh --download

# Build a full bilingual dictionary (includes unmatched titles in both languages)
wikitools pair en zh --download --full

# Extract all English Wikipedia titles as a clickable MDX dictionary
wikitools titles en --download

# Extract all Latin Wiktionary titles
wikitools titles la --project wiktionary --download
```

## Build

Requires Rust 1.75+, and `dictd` package (provides `dictzip` for .dsl.dz compression).

```sh
cargo build --release
```

## Formats

| Subcommand | Format | Output | Reader |
|-----------|--------|--------|--------|
| `pair` | DSL | `.dsl.dz` | ABBYY Lingvo, GoldenDict-ng |
| `titles` | MDX | `.mdx` | MDict, GoldenDict-ng |

`titles` generates an MDX dictionary directly (no intermediate files). A companion `.js` file enables click-to-open-Wikipedia behavior. Place both `.mdx` and `.js` files in the same directory for GoldenDict-ng.

## Data Sources

- **pair**: [Wikidata `wb_items_per_site` dump](https://dumps.wikimedia.org/wikidatawiki/latest/) (~1.8 GB). Only article titles (no Category/Template/Wikipedia namespaces).
- **titles**: Wikimedia `all-titles-in-ns0` dump (~100-200 MB per language). Complete list of page titles for any Wikimedia project (includes redirects).

Dumps are cached at `~/.cache/wikitools/`. Use `--download` to fetch on demand.
