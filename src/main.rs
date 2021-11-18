use clap::{Arg, App, AppSettings, SubCommand};
use koifeed::{Feed, Opml};
use reqwest::blocking::Client;
use std::env;
use std::error::Error;
use std::fs;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = App::new("koifeed")
        .about("Composable CLI for RSS/Atom feeds, written in Rust")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(SubCommand::with_name("content")
            .about("Get the main content of an entry")
            .arg(Arg::with_name("feed")
                 .help("A key by which to search for a feed title")
                 .required(true))
            .arg(Arg::with_name("entry")
                 .help("An integer indexing the desired entry, starting at zero")
                 .required(true)))
        .subcommand(SubCommand::with_name("link")
            .about("Print the homepage link for a feed provider or the link for a specific entry (works with RSS enclosures)")
            .arg(Arg::with_name("feed")
                 .help("A key by which to search for a feed title")
                 .required(true))
            .arg(Arg::with_name("entry")
                 .help("An integer indexing the desired entry, starting at zero")))
        .subcommand(SubCommand::with_name("list")
            .about("List feeds with their dates of last update, or similarly list entries from specific feeds")
            .arg(Arg::with_name("feed")
                 .help("A key by which to search for a feed title"))
            .arg(Arg::with_name("tags")
                 .help("A single, comma-separated argument specifying all feeds with any of the specified tags")
                 .short("t")
                 .long("tags")
                 .conflicts_with("feed")
                 .value_delimiter(","))
            .arg(Arg::with_name("long")
                 .help("Print all entries of each matching feed")
                 .short("l")
                 .long("long")
                 .requires("tags")))
        .subcommand(SubCommand::with_name("update")
            .about("Update the cached feeds; koifeed never does this automatically")
            .arg(Arg::with_name("feed")
                 .help("A key by which to search for a feed to update"))
            .arg(Arg::with_name("tags")
                 .help("A single, comma-separated argument specifying all feeds with any of the specified tags")
                 .short("t")
                 .long("tags")
                 .conflicts_with("feed")
                 .value_delimiter(",")))
        .get_matches();

    let path_home = env::var("HOME").expect("HOME environment variable inaccessible");
    // TODO: attempt to create this directory if missing
    let path_feed_dir = format!("{}/.local/share/koifeed/", path_home);
    // TODO: warn about missing file gracefully
    let opml = fs::read_to_string(format!("{}/.config/koifeed/feeds.opml", path_home)).expect("error accessing opml file");
    let opml = Opml::new(opml)?;

    match arguments.subcommand() {
        ("content", Some(arguments)) => {
            let key = arguments.value_of("feed").unwrap(); // clap-rs guarantees unwrappability
            let index_string = arguments.value_of("entry").unwrap(); // clap-rs guarantees unwrappability
            let index = index_string.parse::<usize>().expect("entry must be specified as a nonnegative integer");
            let title = opml.find(key).expect("invalid feed key");
            let feed = init_feed_by_title(&path_feed_dir, &title)?;
            let contents = feed.contents().nth(index).expect("entry index out of bounds");
            println!("{}", contents);
        },
        ("link", Some(arguments)) => {
            let key = arguments.value_of("feed").unwrap(); // clap-rs guarantees unwrappability
            let title = opml.find(key).expect("invalid feed key");
            match arguments.value_of("entry") {
                Some(index_string) => {
                    let feed = init_feed_by_title(&path_feed_dir, &title)?;
                    let index = index_string.parse::<usize>().expect("entry must be specified as a nonnegative integer");
                    let mut enclosures = feed.enclosure_links().peekable();
                    let link = if enclosures.peek().is_none() {
                        feed.links().nth(index).expect("entry index out of bounds")
                    } else {
                        enclosures.nth(index).expect("entry index out of bounds")
                    };
                    println!("{}", link);
                },
                None => {
                    let feed_titles = opml.titles();
                    let links = opml.links_html();
                    for (feed_title, link) in feed_titles.zip(links) {
                        if title == feed_title {
                            println!("{}", link.expect("no HTML link found for that feed"));
                            break;
                        }
                    }
                },
            }
        },
        ("list", Some(arguments)) => {
            match (arguments.value_of("feed"), arguments.is_present("tags")) {
                (Some(key), false) => {
                    let title = opml.find(key).expect("invalid feed key");
                    println!("feed: {}", title);
                    let feed = init_feed_by_title(&path_feed_dir, &title)?;
                    let dates = feed.dates();
                    let titles = feed.titles();
                    //               yyyy-mm-dd
                    println!("INDEX\tDATE      \tTITLE");
                    for (index, (date, title)) in dates.zip(titles).enumerate() {
                        println!("{}\t{}\t{}", index, date, title);
                    }
                },
                (None, true) => {
                    let search_tags = arguments
                        .values_of("tags")
                        .unwrap();
                    let tags = opml.tags();
                    let titles = opml.titles();
                    let long_flag_set = arguments.is_present("long");
                    if !long_flag_set {
                        //        yyyy-mm-dd
                        println!("DATE      \tFEED");
                    }
                    for (title, tags) in titles.zip(tags) {
                        //println!("DEBUG: title: {}; tags: {:?}", title, tags);
                        for search_tag in search_tags.clone() {
                            let search_string = String::from(search_tag);
                            if tags.contains(&search_string) {
                                let feed = init_feed_by_title(&path_feed_dir, &title)?;
                                let dates = feed.dates();
                                if arguments.is_present("long") {
                                    println!("feed: {}", title);
                                    println!("-----");
                                    //               yyyy-mm-dd
                                    println!("INDEX\tDATE      \tTITLE");
                                    let entry_titles = feed.titles();
                                    for (index, (date, entry_title)) in dates.zip(entry_titles).enumerate() {
                                        println!("{}\t{}\t{}", index, date, entry_title);
                                    }
                                    println!();
                                } else {
                                    let feed = init_feed_by_title(&path_feed_dir, &title)?;
                                    let date = feed.dates().next().expect("no entries found");
                                    println!("{}\t{}", date, title);
                                    break;
                                }
                            }
                        }
                    }
                },
                (None, false) => { // list all feeds
                    let titles = opml.titles();
                    //        yyyy-mm-dd
                    println!("DATE      \tTITLE");
                    for title in titles {
                        let feed = init_feed_by_title(&path_feed_dir, &title)?;
                        let date = feed
                            .dates()
                            .next()
                            .expect("no entries found");
                        println!("{}\t{}", date, title);
                    }
                },
                _ => {
                    /* clap ensures that the (None, true) case will error immediately because
                     * "feed" and "tags" are conflicting arguments.
                     */
                }
            }
        },
        ("update", Some(arguments)) => {
            /*
            .arg(Arg::with_name("feed")
            .arg(Arg::with_name("tags")
                 .conflicts_with("feed")
                 .value_delimiter(",")))
             */
            let client = Client::new();
            match (arguments.value_of("feed"), arguments.is_present("tags")) {
                (Some(key), false) => {
                    let search_title = opml.find(key).expect("invalid feed key");
                    let titles = opml.titles();
                    let links_xml = opml.links_xml();
                    for (title, link_xml) in titles.zip(links_xml) {
                        if title == search_title {
                            match client.get(&link_xml).send() {
                                Ok(response) => match response.text() {
                                    Ok(string) => {
                                        fs::write(format!("{}/{}", path_feed_dir, title), string)?;
                                    },
                                    Err(_) => {
                                        eprintln!("error: empty response from {}", link_xml);
                                    },
                                },
                                Err(_) => {
                                    eprintln!("error: request failure for {}", link_xml);
                                },
                            }
                            break;
                        }
                    }
                },
                (None, true) => {
                    let given_tags: Vec<_> = arguments.values_of("tags").unwrap().collect();
                    let titles = opml.titles();
                    let links_xml = opml.links_xml();
                    let tag_lists = opml.tags();
                    for ((title, link_xml), tags) in titles.zip(links_xml).zip(tag_lists) {
                        for tag in tags {
                            if given_tags.contains(&&tag[..]) {
                                eprintln!("updating {}", title);
                                match client.get(&link_xml).send() {
                                    Ok(response) => match response.text() {
                                        Ok(string) => {
                                            fs::write(format!("{}/{}", path_feed_dir, title), string)?;
                                        },
                                        Err(_) => {
                                            eprintln!("error: empty response from {}", link_xml);
                                        },
                                    },
                                    Err(_) => {
                                        eprintln!("error: request failure for {}", link_xml);
                                    },
                                }
                                break;
                            }
                        }
                    }
                },
                (None, false) => {
                    let titles = opml.titles();
                    let links_xml = opml.links_xml();
                    for (title, link_xml) in titles.zip(links_xml) {
                        eprintln!("updating {}", title);
                        let body = match client.get(&link_xml).send() {
                            Ok(response) => match response.text() {
                                Ok(string) => string,
                                Err(_) => {
                                    eprintln!("\terror: empty response from {}", link_xml);
                                    continue;
                                },
                            },
                            Err(_) => {
                                eprintln!("\terror: request failure for {}", link_xml);
                                continue;
                            },
                        };
                        fs::write(&format!("{}/{}", path_feed_dir, title), body)?;
                    }
                },
                _ => {}, // all other cases have already been handled by clap-rs.
            }
        },
        _ => {}, // should never be hit
    }

    Ok(())
}

fn init_feed_by_title(dir_path: &str, title: &str) -> Result<Feed, Box<dyn Error>> {
    let feed_path = format!("{}/{}", dir_path, title);
    let text = fs::read_to_string(feed_path)?;
    Ok(Feed::new(text))
}
