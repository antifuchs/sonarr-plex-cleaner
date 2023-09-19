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

You need to have Sonarr and a media server (Plex or Jellyfin) running.
From them, you'll need:

* Your Sonarr API's base URL. This is usually the URL that you use to reach Sonarr, plus `/api`.
* Your Sonarr API key. You can find it in `Settings -> General`.

If you're running Plex,
* Your Plex server's URL. Use the one that you use to reach the Plex Web server.
* Your Plex API key. Find it via this article: https://support.plex.tv/articles/204059436-finding-an-authentication-token-x-plex-token/

If you're running emby or jellyfin (I have only tested with the latter),
* Your jellyfin server's URL. Use the base URL that you use to reach the jellyfin server on.
* A jellyfin API token. An admin can make one for you
* The username whose watched states you want to consider.

## Planning to use this tool

By default, this tool does *not* make any changes, unless you pass the
`-f` / `--delete-files` command line parameter. Don't do that and your
data will be safe. In the default mode, the CLI will only output what
seasons it would clean up.

If you wish to prevent the CLI from deleting a show you want to keep
around, tag the show (you can do this in sonarr's "Edit" screen for
the show). You can use a tag named `retain` to indicate that the show
is manually managed.

## The configuration file

`sonarr-plex-cleaner` reads all this data from a configuration file;
on Linux, it lives in `~/.config/sonarr-plex-cleaner.yaml`; on macOS,
it lives in `~/Library/Preferences/sonarr-plex-cleaner.toml`. Create
the appropriate file for your platform, with contents like the
following:

``` toml
[tv]
url = "https://sonarr.example.com/api/"  # Your sonarr installation's API URL
api_key = "deadbeef5ec9e7"               # sonarr API key

# Either [plex] or [jellyfin] - delete the one that doesn't apply to you:
[plex]
url = "http://plex.example.com:32400/"   # Your plex API URL
api_key = "deadbeef5ec9e7"               # Plex API key
[jellyfin]
url = "http://jellyfin.example.com:8096/" # your jellyfin API URL
api_key = "aaaaaaaaaaaaaaaaaaaa"          # Jellyfin API key
user = "your_username"                    # User to consider for watched states

[retention]
# Tag that marks a show as manually managed
retain_tag = "retain"

# Wait 14 days after last air date before deleting even a completely watched show:
retain_duration = "14d"
```

## Usage

You've collected the four items from prerequisites, made the
configuration file and have tagged the shows that you wish to
keep. Great, let's see what it would delete:

``` sh
sonarr-plex-cleaner tv
```

which will output something like:

```
INFO [sonarr_plex_cleaner] delete 10 files: Piracy On The High Seas S03: 9.64 GiB
INFO [sonarr_plex_cleaner] delete 7 files: Piracy On The High Seas S04: 9.05 GiB
```

### Actually deleting files

Run:

``` sh
sonarr-plex-cleaner tv --delete-files
```

to unmonitor each of the seasons above in Sonarr, and delete the files in that season.
