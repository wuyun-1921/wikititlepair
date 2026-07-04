# wikitools: Project Rename + Titles Feature

Date: 2026-07-04

## Summary

Rename `wikititlepair` to `wikitools`.
Move existing pair-generation feature under `pair` subcommand.
Add new `titles` subcommand: extract all article titles from a Wikimedia project dump and emit a DSL dictionary where each entry is a link to the live page.
Bare `wikitools` prints CLI help.

## Rename surface

| What | Before | After |
|------|--------|-------|
| Repo dir | `~/repos/wikititlepair` | `~/repos/wikitools` |
| Cargo package | `wikititlepair` | `wikitools` |
| Binary | `wikititlepair` | `wikitools` |
| Cache dir | `~/.cache/wikidict` | `~/.cache/wikitools` |
| Pair output prefix | `wikipedia-titlepair-{lang_a}-{lang_b}-{date}` | unchanged |
| Titles output prefix | — | `wikipedia-titles-{lang}-{date}` |
| DSL name header (pair) | `wikipedia titlepair (en-zh)` | unchanged |
| DSL name header (titles) | — | `wikipedia titles (en)` |

## CLI

```
wikitools                                    # prints help
wikitools pair <lang_a> <lang_b> [--download] [--output <path>] [--cache-dir <path>]
wikitools titles <lang> [--project <project>] [--download] [--output <path>] [--cache-dir <path>]
```

Implemented with clap `#[derive(Subcommand)]` enum.
`pair` and `titles` have independent arg sets — no flag leakage between subcommands.

### Flags

- `--download` — fetch dump if not cached. Both subcommands default to no-download: error if dump missing, `--download` to fetch.
- `--output <path>` — override output file path. Has sensible default based on feature, language(s), and dump date.
- `--cache-dir <path>` — override cache directory. Default: `~/.cache/wikitools/`.
- `--project <project>` (titles only) — Wikimedia project. Default: `wikipedia`. Valid: `wikipedia`, `wiktionary`, `wikibooks`, `wikiquote`, `wikisource`, `wikinews`, `wikiversity`, `wikivoyage`.

## Data flow — `pair` (unchanged)

1. Download Wikidata `wb_items_per_site` dump (if `--download` and not cached)
2. Parse INSERT rows for both `{lang}wiki` sites
3. Build bidirectional pairs: match item_id across languages
4. Skip identical titles (case-insensitive), non-article namespace pages
5. Sort, deduplicate
6. Write DSL, compress with dictzip

## Data flow — `titles`

1. Download `{lang}{project}-latest-all-titles-in-ns0.gz` from `https://dumps.wikimedia.org/{lang}{project}/latest/` (if `--download` and not cached)
2. Stream gunzip, parse XML for `<page>` blocks, extract `<title>` text
3. Skip non-article namespace titles (same `NON_ARTICLE_PREFIXES` list as `pair`)
4. For each title, construct URL: `https://{lang}.{project}.org/wiki/{Title}`
   - Spaces replaced with underscores
   - RFC 3986 percent-encoding for special characters
5. DSL entry format:
   ```
   Title
   	<a href="https://en.wikipedia.org/wiki/Title">https://en.wikipedia.org/wiki/Title</a>
   ```
6. Sort, deduplicate
7. Write DSL header, body, compress with dictzip

## Source modules

```
src/
  main.rs          # CLI definition + subcommand dispatch (~50 lines)
  error.rs         # WikiDictError enum
  escape.rs        # escape_dsl(), unquote() — shared across features
  dsl.rs           # DslWriter: header + entry writing
  download.rs      # ensure_dump(), download_file(), get_dump_date()
  pair.rs          # parse_dump(), parse_insert_line() — Wikidata path
  titles.rs        # parse_all_titles(), XML title extraction, URL construction
```

`main.rs` shrinks from ~650 lines to ~50 lines of clap structs + dispatch.
Each module has a single clear responsibility, testable in isolation.

## GitHub Actions

### `update-dictionary.yml`

- Rename binary references: `wikititlepair` → `wikitools`
- Rename cache keys: `wikidata-dump-*` → `wikitools-wikidata-*`
- Add `titles` job for en titles + zh titles dictionaries
- Add titles cache key: `wikitools-titles-{lang}-{date}`
- Cache path: `~/.cache/wikitools`
- Upload titles DSL artifacts alongside pair artifacts

### `release.yml`

- Rename binary references: `wikititlepair` → `wikitools`
- Package artifact names: `wikitools-{target}` instead of `wikititlepair-{target}`
- Add titles dictionaries to release bundle
