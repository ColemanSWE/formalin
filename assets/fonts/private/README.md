# Private fonts (optional)

Put your own font files here to replace the bundled open-licensed fonts in
**your local builds**. This folder's contents are git-ignored, so they're
never committed.

| File | Replaces | Used for |
|---|---|---|
| `Corepix.ttf` | `../PirataOne-Regular.ttf` | display text (title, readouts) |
| `Esoterica.ttf` | `../NotoSansSymbols.ttf` | the symbols (sigils) |

Both files must be present; `build.rs` checks for them. Delete them (or run
`cargo clean`) to go back to the bundled fonts. Don't distribute builds made
with fonts you don't have the rights to redistribute.
