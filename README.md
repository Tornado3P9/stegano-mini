# stegano-mini

Steganography tool for PNG file format.

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

```bash
stegano-mini embed -c cover-image.png -e secret.txt -o image.png
```

<!-- ```bash
Embed data into a PNG image file

Usage: stegano-mini embed [OPTIONS] --coverfile <COVERFILE> --embedfile <EMBEDFILE>

Options:
  -c, --coverfile <COVERFILE>    Path to the cover PNG image file
  -e, --embedfile <EMBEDFILE>    Path to the file to embed
  -o, --outputfile <OUTPUTFILE>  Optional path to the output PNG image file [default: output.png]
  -h, --help                     Print help
``` -->

## Extract

```bash
stegano-mini extract -s image.png -o secret.png
```

<!-- ```bash
Extract data from a PNG image file

Usage: stegano-mini extract [OPTIONS] --stegofile <STEGOFILE>

Options:
  -s, --stegofile <STEGOFILE>    Path to the stego PNG image file that holds the secret data
  -o, --outputfile <OUTPUTFILE>  Optional path to the output file [default: output.txt]
  -h, --help                     Print help
```

## Help

```bash
stegano-mini
stegano-mini -h
stegano-mini --help
stegano-mini help

stegano-mini embed -h
stegano-mini embed --help
stegano-mini help embed

stegano-mini extract -h
stegano-mini extract --help
stegano-mini help extract
``` -->
