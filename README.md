# rsst

An Atom/RSS reader written in Rust. This is a personal project, but if you're interested in some portability-related changes, let me know.

## Some Basic Questions

### What
A barebones aggregator for the command line emphasizing composability.

### Why
I love the terminal for several reasons: how applications can easily interact, how few machine resources are necessary, and its general retro aura. I used to use [Newsboat](https://github.com/newsboat/newsboat), but it seems to lack documentation or implementation for features I want. It also contained features I don't need. I found it more difficult than preferred to pull the information I wanted from specific feeds and articles and to feed that information to other programs.

## Dependencies (see `Cargo.toml`)
- cargo
	- reqwest
	- xml-rs
- probably some others that I forgot about

## Usage

### Setup
From source only at the moment. Feeds are stored directly in an OPML file. The program reads this file to update individual feed files named by title. There is currently no way to manage feeds other than editing the OPML file directly. There are no plans to add any. `rsst` requires that this file be located at `$HOME/.config/feeds.opml`; it stores feeds at `$HOME/.local/share/rss`.

### Commands
Replace occurrences of "#" with the index of the desired entry. Feeds are specified by a key that will match its first superstring.
Command | Effect
--------|-------
`rsst update` | Update all feed files. Displays indices, feed titles, and completion status as individual feeds are updated. This is the only function that requires an internet connection.
`rsst list` | List index, date of most recent update, and title for each feed. Indexing starts at 0 and is done automatically.
`rsst list #` | List index, date of most recent update, and title for each entry in the given feed.
`rsst content feed #` | Print the main content of the given entry (#) in the given feed ('feed'). May contain raw HTML (see "Examples").
`rsst link #` | Print the main link given for the given feed, often to a homepage.
`rsst link feed #` | Print the main link given for the given entry in the given feed. Not all entries contain links.

Example | Effect
--------|-------
`rsst list \| grep 01-01` | Print a list of all feeds that were published/last updated on January 1st.
`rsst content favnewsfeed 1 \| w3m -T text/html` | Assuming the second article in the second feed contains raw HTML, page through the properly displayed HTML using w3m.
`` mpv `rsst link youtubefeed 0` `` | Use mpv with youtube-dl to play the latest video from a youtube-generated feed.
