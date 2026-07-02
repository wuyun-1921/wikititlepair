# DSL Format Reference for wikititlepair

ABBYY Lingvo DSL format specification as it applies to Wikipedia title pairs.

## Heading Character Rules

DSL treats certain characters in headwords as markup. They must be backslash-escaped to appear as literal text.

| Char | DSL meaning | Escape | Example |
|------|-------------|--------|---------|
| `\` | Escape character | `\\` | `A\B` → `A\\B` |
| `(` `)` | Optional part markers | `\(` `\)` | `MUSIC (algorithm)` → `MUSIC \(algorithm\)` |
| `{` `}` | Unsorted part markers | `\{` `\}` | `foo {bar}` → `foo \{bar\}` |
| `[` `]` | Forbidden in heading | `\[` `\]` | `[test]` → `\[test\]` |
| `#` | Forbidden in heading | `\#` | `C#` → `C\#` |
| `@` | Forbidden in heading | `\@` | `user@host` → `user\@host` |
| `<` `>` | Forbidden in heading | `\<` `\>` | `A < B` → `A \< B` |
| `~` | Forbidden in first heading | `\~` | `x ~ y` → `x \~ y` |
| `^` | Forbidden in first heading | `\^` | `x^2` → `x\^2` |

Characters NOT requiring escape: `&`, `'`, `"`, `-`, `_`, `,`, `.`, `/`, `:`, `!`, `?`, `*`, `+`, `=`, `|`, `$`, `%`, `;`.

## How These Rules Were Found

**Parentheses `()`** — Mark optional (alternative) parts of a headword. `MUSIC (algorithm)` creates a headword with optional part `(algorithm)`, causing GoldenDict/GoldenDict-NG to index it under `MUSIC` without the parenthesized part. PyGlossary completely strips the optional part during conversion. Escape with `\(` `\)`.

**Curly braces `{}`** — Mark unsorted parts of a headword. The braced portion is ignored for sorting and not shown in the word list. `{to }have` sorts under "have" but displays as "to have". Escape with `\{` `\}`.

**Square brackets `[]`** — Officially impossible to use in the heading. Escape with `\[` `\]`.

**Hash `#`, at `@`, angle brackets `< >`** — Disallowed in the heading. Escape with backslash.

**Tilde `~` and caret `^`** — Forbidden in the first heading of a card (but allowed in subentry headings). Escape with `\~` `\^`.

**Backslash `\`** — The escape character itself. Must be doubled `\\` to appear as literal.

## DSL Structure

A DSL dictionary is a plain text file (UTF-8 or ANSI encoding) with the following structure:

```
#NAME "Dictionary Name"
#INDEX_LANGUAGE "SourceLanguage"
#CONTENTS_LANGUAGE "TargetLanguage"

headword
 body text (must start with space or tab)
 next headword
 body text
```

Key structural rules:
- Headword must start at column 0 (first position of line)
- Body text must start with space or tab
- Empty lines between entries allowed
- No two entries can have the same heading (case-sensitive: `Music` and `music` are distinct)

## Cross-references

Use `<<word>>` to create clickable links between entries:

```
Music
	<<音乐>>
音乐
	<<Music>>
```

The referenced word must match the escaped headword exactly. If the headword is `C\#`, the reference must be `<<C\#>>`.

## Tilde substitution

In card body text, `~` substitutes the headword. Not used in wikititlepair.

## Sources

- [ABBYY Lingvo DSL Dictionary Structure](http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/dsl-dictionary-structure/)
- [DSL Tags](http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/dsl-tags/)
- [Optional Part of an Entry Headword](http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/optional-part-of-an-entry-headword/)
- [Unsorted Part of an Entry Headword](http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/unsorted-part-of-an-entry-headword/)
- [How Entries Are Sorted](http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/how-entries-are-sorted/)
- [DSL Commands](https://documentation.help/ABBYY-Lingvo8/dsl_commands.htm)
- [Structure of DSL Card](https://documentation.help/ABBYY-Lingvo8/EntryStructure.htm)
- [How Headers Are Sorted](https://documentation.help/ABBYY-Lingvo8/head_sort.htm)
