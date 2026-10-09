# Command-Line Help for `dominant-colour`

This document contains the help content for the `dominant-colour` command-line
program.

**Command Overview:**

- [`dominant-colour`↴](#dominant-colour)
- [`dominant-colour generate`↴](#dominant-colour-generate)

## `dominant-colour`

Generate dominant colour, base64 data-URIs for use as website image placeholders

**Usage:** `dominant-colour [OPTIONS] <COMMAND>`

###### **Subcommands:**

- `generate` — Generate a placeholder

###### **Options:**

- `-v`, `--verbose` — Increase logging verbosity
- `-q`, `--quiet` — Decrease logging verbosity

## `dominant-colour generate`

Generate a placeholder

**Usage:** `dominant-colour generate [OPTIONS] <INPUT>`

###### **Arguments:**

- `<INPUT>` — Path to input image file

###### **Options:**

- `-o`, `--output <OUTPUT>` — Write generated image to path
- `-c`, `--clipboard` — Copy generated URI to clipboard
- `--svg` — Create an SVG, instead of preserving the input format

<hr />

<small><i>
This document was generated automatically by
<a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
