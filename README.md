# sonarr-plex-cleaner: Garbage-collect your TV broadcatching library

If you use Sonarr, you might be pleased by how good it is at
downloading data. Unfortunately, it's not as good at cleaning up that
data you no longer need (somewhat because sonarr doesn't know when you
no longer need that data!).

This tool exists to fill that gap: It queries both sonarr (the thing
downloading media) and plex (the thing keeping track of whether you
watched that media), and deletes everything that has been fully
downloaded and watched.

## Installation

1. Get a rust installation: https://rustup.rs/
2. Install the CLI tool in this repo via cargo: `cargo install --git=https://github.com/antifuchs/sonarr-plex-cleaner.git`

## Prerequisites

You need to have Sonarr and Plex running. From them, you'll need:

* Your Sonarr API's base URL. This is usually the URL that you use to reach Sonarr, plus `/api`.
* Your Plex server's URL. Use the one that you use to reach the Plex Web server.
* Your Sonarr API key. You can find it in `Settings -> General`.
* Your Plex API key. Find it via this article: https://support.plex.tv/articles/204059436-finding-an-authentication-token-x-plex-token/

## Planning to use this tool

By default, this tool does *not* make any changes, unless you pass the
`-f` / `--delete-files` command line parameter. Don't do that and your
data will be safe. In the default mode, the CLI will only output what
seasons it would clean up.

If you wish to prevent the CLI from deleting a show you want to keep
around, tag the show (you can do this in sonarr's "Edit" screen for
the show). By default, the tag name `retain` will prevent the show
from being garbage-collectable.

## Usage

You've collected the four items from prerequisites, and have tagged
the shows that you wish to keep. Great, let's see what it would
delete:

``` sh
sonarr-plex-cleaner --sonarr=https://sonarr.example.com/api/ --sonarr-api-key=ffffffffffff --plex http://plex.example.com:32400/ --plex-api-key=fffffffffffff
```

which will output something like:

```
INFO [sonarr_plex_cleaner] delete 10 files: Piracy On The High Seas S03: 9.64 GiB
INFO [sonarr_plex_cleaner] delete 7 files: Piracy On The High Seas S04: 9.05 GiB
```

### Actually deleting files

Run:

``` sh
sonarr-plex-cleaner --sonarr=https://sonarr.example.com/api/ --sonarr-api-key=ffffffffffff --plex http://plex.example.com:32400/ --plex-api-key=fffffffffffff --delete-files
```

to unmonitor each of the seasons above in Plex, and delete the files in that season.
