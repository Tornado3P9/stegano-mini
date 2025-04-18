# stegano-mini

```bash
Stegano-Mini

Usage: stegano-mini <COMMAND>

Commands:
  embed    Embed data into a PNG image file
  extract  Extract data from a PNG image file
  help     Print this message or the help of the given subcommand

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Embed

`stegano-mini embed -c image.png -e secret.txt`

<!-- ```bash
Embed data into a PNG image file

Usage: stegano-mini embed --coverfile <COVERFILE> --embedfile <EMBEDFILE>

Options:
  -c, --coverfile <COVERFILE>  Path to the cover PNG image file
  -e, --embedfile <EMBEDFILE>  Path to the file to embed
  -h, --help                   Print help
``` -->

## Extract

`stegano-mini extract -s secret.png`

<!-- ```bash
Extract data from a PNG image file

Usage: stegano-mini extract --stegofile <STEGOFILE>

Options:
  -s, --stegofile <STEGOFILE>  Path to the stego PNG image file that holds the secret data
  -h, --help                   Print help
``` -->

## Help

```bash
stegano-mini
stegano-mini -h
stegano-mini --help
stegano-mini help

stegano-mini help embed
stegano-mini embed -h
stegano-mini embed --help
```
