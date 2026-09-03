# image-metadata-fmt

I keep a `.imeta` sidecar file next to photos I care about (title, tags,
who took it, when) instead of trusting whatever's baked into the JPEG.
Problem is I write these by hand, sometimes from a phone, sometimes I
paste in output from some other tool, and they end up inconsistent:
`:` in one line and `=` in the next, quoted strings in one place and bare
words in another, trailing commas in tag lists, dates like `2023-3-4`
instead of `2023-03-04`. None of that is wrong exactly, it's just not
uniform, and diffs between files get noisy.

This is a formatter for that file. It reads a loose `.imeta` file and
prints a canonical version: lower_snake_case keys, quoted scalars,
bracketed tag lists, zero-padded dates. It doesn't touch the actual image
files or their embedded EXIF/IPTC data - just the sidecar text.

## Example

Input (`photo.imeta`):

```
title: "Sunset over the bay"
tags: beach, sunset,  golden hour,
Artist:mike
Date Taken: 2023-3-4
copyright = "(c) 2023 Mike"
```

```
$ image-metadata-fmt photo.imeta
title = "Sunset over the bay"
tags = ["beach", "sunset", "golden hour"]
artist = "mike"
date_taken = 2023-03-04
copyright = "(c) 2023 Mike"
```

Key names are case- and separator-insensitive on input (`Date Taken`,
`date-taken`, and `date_taken:` all land on the same canonical key), tag
lists get trimmed and their trailing comma dropped, and dates get
zero-padded rather than reformatted into something unrecognizable.

## Error messages

The thing I actually wanted from this tool: when a file is broken, say
exactly where.

Input with a missing closing quote:

```
title: "Sunset over the bay
tags: beach, sunset
```

```
$ image-metadata-fmt broken.imeta
error: unterminated string literal
  |
1 | title: "Sunset over the bay
  |        ^
  at line 1, column 8
```

Every parse error carries a line, a column, and the actual source line so
you don't have to go count characters yourself. Column numbers are in
characters, not bytes, so they stay correct on lines with accented
letters or other multi-byte UTF-8.

## Format

An `.imeta` file is line-oriented. Each non-blank, non-comment line is
`key: value` or `key = value`. Lines starting with `#` are comments and
blank lines are ignored.

A quoted value that isn't closed on its own line is taken to continue on
the following lines, up to the line that closes it:

```
notes: "shot on the ferry,
came out grainier than I wanted"
```

normalizes to a single line, with the break kept as `\n`:

```
notes = "shot on the ferry,\ncame out grainier than I wanted"
```

- `tags` / `keywords` - comma-separated list, becomes a bracketed list
- `date_taken` / `date` - `Y-M-D` with any of `-`, `/`, `.` as the
  separator, becomes zero-padded `YYYY-MM-DD`
- anything else - treated as a quoted scalar string

A `date_taken` value that isn't three numeric parts, or has a month or day
out of range, is rejected rather than passed through - it gets the same
line/column error treatment as a parse failure:

```
$ image-metadata-fmt bad-date.imeta
error: 'next tuesday' is not a valid date, expected Y-M-D
  |
1 | date_taken: next tuesday
  |             ^
  at line 1, column 13
```

## Usage

```
image-metadata-fmt <file.imeta>
image-metadata-fmt <directory>
```

Normalized output goes to stdout; parse errors go to stderr with a
non-zero exit code.

Pointed at a directory, it normalizes every `.imeta` file directly inside
it (not recursive), printing a `== path ==` header before each one. A
parse error in one file doesn't stop the rest - everything that parses
still gets printed, and the process exits non-zero if anything failed.

## Status

Early. The known fields (`tags`, `keywords`, `date_taken`, `date`) are
validated against their expected shape; anything else is treated as a
freeform string with no validation. There's no `--check` or `--write`
mode yet - it only ever prints the normalized form to stdout.

## Building

Standard library only, no external crates.

```
cargo build --release
```

## License

MIT, see LICENSE.
