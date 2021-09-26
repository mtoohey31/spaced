# `spaced`

Command line, spaced repetition software using markdown.

![screenshot](https://user-images.githubusercontent.com/36740602/134814105-44cbda7e-5dd2-4c90-a480-7aab2d0b5731.png)

## Usage

```sh
spaced c
spaced cards c # equivalent
spaced cards clear-cards # equivalent

spaced cards clear-cards --no-confirm

spaced n

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
