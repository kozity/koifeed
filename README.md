# koifeed

An Atom/RSS reader written in Rust. This is a personal project, but if you're interested in some portability-related changes, let me know.

## Some Basic Questions

### What
A barebones feed aggregator for the command line emphasizing composability.

### Why
I love the terminal for several reasons: how applications can easily interact, how few machine resources are necessary, and its general retro aura. I used to use [Newsboat](https://github.com/newsboat/newsboat), but it seems to lack documentation or implementation for features I want. It also contained features I don't need. I found it more difficult than preferred to pull the information I wanted from specific feeds and articles and to feed that information to other programs. Still, many of its users swear by it, so the fault probably lied with me there. In any case, this program is fun to write and maintain, and it works the way I want it to.

## Dependencies (see `Cargo.toml`)
- clap-rs
- reqwest
- xml-rs
- probably some others that I forgot about

## Usage

### Setup
`cargo install koifeed` should work. Otherwise, building from source should work on most Unix-like systems. Feeds are stored directly in an OPML file. The program reads this file to update individual feed files named by title. There is currently no way to manage feeds other than editing the OPML file directly. There are no plans to add any. `koifeed` requires that this file be located at `$HOME/.config/koifeed/feeds.opml`; it stores feeds in `$HOME/.local/share/koifeed/`.

### Commands
Command-line arguments are handled by the glorious [`clap-rs`][clap] crate. `clap` provides the `--help` flag for `koi` as well as all of its subcommands.

### Examples
Example | Effect
--------|-------
`koi list \| grep 01-01`                        | Print a list of all feeds that were published/last updated on January 1st.
`koi content favnewsfeed 1 \| w3m -T text/html` | Assuming the second article in the second feed contains raw HTML, page through the properly displayed HTML using w3m.
`` mpv `koi link youtubefeed 0` ``              | Use mpv with youtube-dl to play the latest video from a youtube-generated feed.

[clap]: https://clap.rs/
