# `spaced`

Command line, spaced repetition software using markdown.

![screenshot](https://user-images.githubusercontent.com/36740602/134814105-44cbda7e-5dd2-4c90-a480-7aab2d0b5731.png)

## Features

- Multiple algorithms:
  - Leitner
  - All
  - More coming soon...
- Algorithms use [event sourcing](https://en.wikipedia.org/wiki/Domain-driven_design#Event_sourcing) to determine review time so no algorithm-specific data needs to be stored in cards.
- Imports from:
  - [Mochi](https://mochi.cards)
  - [Anki](https://apps.ankiweb.net) (currently rudimentary)
  - Please [open an issue](https://github.com/mtoohey31/spaced/issues/new) if you'd like to request an import format.
- Undo!

## Usage

```sh
spaced c
spaced cards c # equivalent
spaced cards clear-cards # equivalent

spaced cards clear-cards --no-confirm

spaced n
spaced notes # equivalent

spaced
spaced r # equivalent
spaced review # equivalent
spaced review . --algorithm leitner # equivalent

spaced review .. --algorithm all

spaced i -f mochi export.mochi cards/
spaced import --format mochi export.mochi cards/ # equivalent

spaced import --format anki export.apkg cards/
spaced import --format anki export.colpkg cards/
```

Refer to `-h` argument or `help` subcommand for further information.
