# rsst

An RSS reader written in Rust. This is a personal project not intended for library use, but if you're interested in that sort of thing, feel free to open an issue.

## Some Basic Questions
### What
A barebones rss aggregator for the terminal that tries its best to obey UNIX philosophy.
### Why
I love the terminal for several reasons: how applications can easily interact, how few machine resources are necessary, and its general retro aura. I used to use [Newsboat](https://github.com/newsboat/newsboat), but it seems to lack documentation or implementation for features I want. It also contained features I don't need. I found it more difficult than preferred to pull the information I wanted from specific feeds and articles and to feed that information to other programs.
### How (Meaning "how I made it". For usage, see "Usage" below)
Some Rust from someone who didn't know much Rust before writing this.

## Dependencies
- cargo
- probably some others that I forgot about

## Usage
### Setup
Installing with cargo is easiest. As I have it set up on my system, feeds are stored directly in an OPML file (see opml.xml). The program reads this file to update individual feed files named by title. These feed files don't carry a ".xml" extension, but they probably should. There is currently no way to manage feeds other than editing the OPML file directly. There are no plans to add one. Filepaths are currently hard-coded, which is probably not what you want. Open an issue if you want me to fix this.

### Commands
Replace occurrences of "#" with indices of desired feeds or entries. There is currently no support for accessing feeds or entries by title.
Command | Effect
--------|-------
`reader update` | Update all feed files. Displays indices, feed titles, and completion status as individual feeds are updated. This is the only function that requires an internet connection.
`reader list` | List index, date of most recent update, and title for each feed. Indexing starts at 0 and is done automatically.
`reader list #` | List index, date of most recent update, and title for each entry in the given feed.
`reader content # #` | Print the main content of the given entry (second #) in the given feed (first #). May contain raw HTML (see "Examples").
`reader link # #` | Print the main link given for the given entry in the given feed. Not all entries contain links.

Example | Effect
--------|-------
`rsst list \| grep 01-01` | Print a list of all feeds that were published/last updated on January 1st.
`rsst content 1 1 \| w3m -T text/html` | Assuming the second newest article in the second feed contains raw HTML, page through the properly displayed HTML using w3m.
`` mpv `rsst link 11 0` `` | Use mpv with youtube-dl to play the latest video from the youtube-generated feed with index 11.
