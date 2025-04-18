# stegano-mini

### `stegano-mini help`
```bash
Stegano-Mini

Usage: stegano-mini <COMMAND>

Commands:
  embed    Embed data into a PNG image file
  extract  Extract data from a PNG image file
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### `stegano-mini help embed`
```bash
Embed data into a PNG image file

Usage: stegano-mini embed --coverfile <COVERFILE> --embedfile <EMBEDFILE>

Options:
  -c, --coverfile <COVERFILE>  Path to the cover PNG image file
  -e, --embedfile <EMBEDFILE>  Path to the file to embed
  -h, --help                   Print help
```

### `stegano-mini help extract`
```bash
Extract data from a PNG image file

Usage: stegano-mini extract --stegofile <STEGOFILE>

Options:
  -s, --stegofile <STEGOFILE>  Path to the stego PNG image file that holds the secret data
  -h, --help                   Print help
```
