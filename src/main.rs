// TODO: consider using rust's Path trait to work with paths more robustly
use reqwest::blocking::Client;
use std::env;
use std::fs::{self, File};
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};

fn main() -> Result<(), RsstError> {
    // Config is the struct that handles arguments and general flow
    let config = Config::new(
        String::from("/home/ty/.config/rss/opml.xml"), //opml
        String::from("/home/ty/.config/rss/"), //save_dir
    );

    match &config.subcommand {
        Some(sub) => {
            match &sub[..] {
                "content" => config.content()?,
                "link" => config.link()?,
                "list" => config.list()?,
                "update" => config.update()?,
                _ => config.print_usage(),
            }
        },
        None => config.print_usage(),
    }

    Ok(())
}

pub struct Config {
    // path to manually maintained file about feeds in .opml format
    pub opml: String,
    // path to save directory for pulled rss/atom feeds
    pub save_dir: String,
    // name used to call program; used for "Usage:" message
    pub invocation: Option<String>,
    pub subcommand: Option<String>,
    // contains consistent reqwest client. Only used for "update", but keeping a persistent client
    // can cut down on total time for multiple requests
    pub client: Option<Client>,
    pub index_feed: Option<u8>,
    pub index_entry: Option<u8>,
}
impl Config {
    //TODO: validate indices?
    pub fn new(opml: String, save_dir: String) -> Self {
        let mut args = env::args();
        let invocation = args.next();
        let subcommand = args.next();
        // these two indices are so named because they must yet be parsed to integers
        let index_feed_string = args.next();
        let index_entry_string = args.next();
        let mut index_feed = None;
        let mut index_entry = None;
        let mut client = None;

        // ensure, no matter what's hardcoded, save_dir has trailing '/'
        let save_dir_suffixed =
            if save_dir.ends_with('/') {
                save_dir
            } else {
                format!("{}/", save_dir)
            }
        ;

        // we only need a reqwest client if we'll be updating feeds; instantiate it now if so
        if let Some(sub) = &subcommand {
            if &sub[..] == "update" {
                client = Some(Client::new());
            }
        }

        // next two "if" blocks parse indices to integers
        if let Some(string) = index_feed_string {
            if let Ok(number) = string.parse::<u8>() {
                index_feed = Some(number);
            }
        }

        if let Some(string) = index_entry_string {
            if let Ok(number) = string.parse::<u8>() {
                index_entry = Some(number);
            }
        }

        Self {
            opml,
            save_dir: save_dir_suffixed,
            invocation,
            subcommand,
            client,
            index_feed,
            index_entry,
        }
    }

    // get the main content from an entry
    pub fn content(&self) -> Result<(), RsstError> {
        // we need both indices, so if the latter is missing, only one was given.
        // TODO: consider printing better error messages, specific to which index is missing
        if let None = self.index_entry {
            return Err(RsstError::IndexAbsent);
        }

        let mut parser = parser_new(&self.opml)?;
        let mut title = String::new();

        // advance through feeds until we're at the right index. Then, return the title attribute
        if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("title"))? {
            title = string;
        } else {
            // either something didn't have a "title" attribute, which isn't allowed in OPML, or we
            // ran off the end of the file, or some more technical error
            println!("Error: feed not found.");
        }

        // switch parsing from opml to an individual feed file
        let mut parser = parser_new(&format!("{}{}", self.save_dir, title))?;
        // no return value needed from these two lines; just position the parser at the first
        // entry, then at its contents. The next parser event should contain the text inside this
        // tag.
        parser_advance(&mut parser, vec!("entry", "item"), self.index_entry.unwrap() + 1, None)?;
        parser_advance(&mut parser, vec!("content", "description"), 1, None)?;
        match &parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) =>
                println!("{}", string),
            _ => eprintln!("Error: contents not found."),
        }
        Ok(())
    } // content()

    // get what is probably the date of most recent update for the given feed. In reality, get the
    // date from the first entry in the feed. This is subjectively almost always correct.
    fn date_get_feed(&self, feed_name: &str) -> Result<String, RsstError> {
        let mut parser = parser_new(&format!("{}{}", self.save_dir, feed_name))?;
        let mut date = String::new();

        // navigate to the first entry
        parser_advance(&mut parser, vec!("entry", "item"), 1, None)?;
        // navigate to next date tag. Next parser event should be text within tag
        parser_advance(&mut parser, vec!("published", "date", "pubDate"), 1, None)?;
        match &parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => date = String::from(string),
            _ => eprintln!("Error: date not found."),
        }
        // date will have to be parsed from one of two formats depending on rss/atom
        Ok(date_parse(&date))
    } // date_get_feed()

    // get a link either of a feed's homepage or of an entry's specific page
    pub fn link(&self) -> Result<(), RsstError> {
        // this function can work with either one or two indices given
        // TODO: consider using RsstError::IndexAbsent here for consistency
        if let None = self.index_feed {
            println!("Error: no index argument(s) found.");
            return Ok(());
        }

        let mut parser = parser_new(&self.opml)?;
        let mut title = String::new();

        // if there's no index entry, get the feed homepage link
        if let None = self.index_entry {
            // get htmlUrl attribute from relevant outline tag in opml
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("htmlUrl"))? {
                println!("{}", string);
            } else {
                println!("Error: link not found.");
            }
            // we are done with this function since we don't care about a specific entry
            return Ok(());
        } else {
            // get title attribute from relevant outline tag in opml
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("title"))? {
                title = string;
            } else {
                println!("Error: feed not found.");
            }
        }

        // now we want to get the link from a specific entry
        let mut parser = parser_new(&format!("{}{}", self.save_dir, title))?;
        // navigate to the relevant entry
        parser_advance(&mut parser, vec!("entry", "item"), self.index_entry.unwrap() + 1, None)?;
        // get the href attribute from the first link tag in this entry
        if let Some(string) = parser_advance(&mut parser, vec!("link"), 1, Some("href"))? {
            println!("{}", string);
        } else {
            // if there was no such attribute, then the link must be text between tags
            match &parser.next() {
                Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => {
                    println!("{}", string);
                }
                _ => {
                    eprintln!("Error: link not found.");
                }
            }
        }
        Ok(())
    } // link()

    // list information of either all feeds or all entries from one feed
    pub fn list(&self) -> Result<(), RsstError> {
        // if we want to list all entries from one feed
        if let Some(index_feed) = self.index_feed {
            let mut parser = parser_new(&self.opml)?;
            let mut title = String::new();
            // get title attribute from relevant feed
            // TODO: better error handling here
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), index_feed + 1, Some("title"))? {
                title = string;
            }
            
            // prepare parser for specific feed file
            let mut parser = parser_new(&format!("{}{}", self.save_dir, title))?;

            // header: general format for each listing
            println!("({}) {}", index_feed, title);
            println!("Entry\tDate\tTitle");

            let mut date: Option<String> = None;
            let mut title: Option<String> = None;
            let mut index = 0;

            loop {
                // navigate to next entry
                parser_advance(&mut parser, vec!("entry", "item"), 1, None)?;
                // navigate to title tag; next parser event should be title text
                parser_advance(&mut parser, vec!("title"), 1, None)?;
                match &parser.next() {
                    Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => title = Some(string.clone()),
                    Ok(XmlEvent::EndDocument) => break,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                    _ => {}
                }

                // get date with the same method as title
                parser_advance(&mut parser, vec!("published", "date", "pubDate"), 1, None)?;
                match &parser.next() {
                    Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => date = Some(date_parse(string)),
                    Ok(XmlEvent::EndDocument) => break,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                    _ => {}
                }

                // a listing is only printed once a title and date are found for the entry. This is
                // in case things are ordered strangely
                if let (Some(title_inner), Some(date_inner)) = (&title, &date) {
                    println!("{}\t{}\t{}", index, date_inner, title_inner);
                    date = None;
                    title = None;
                    index += 1;
                }
            }
        } else {
            // otherwise, we want to list information about all feeds
            let mut parser = parser_new(&self.opml)?;
            let mut index = 0;

            println!("Feed\tUpdated\tTitle");

            loop {
                let title;
                // navigate to next outline tag and act on the title attribute
                match parser_advance(&mut parser, vec!("outline"), 1, Some("title")) {
                    Ok(Some(string)) => title = string,
                    Err(RsstError::Xml(_)) => {
                        eprintln!("\t Error listing feed ({})", index);
                        index += 1;
                        continue;
                    }
                    _ => break,
                }
                println!("{}\t{}\t{}", index, self.date_get_feed(&title)?, title);
                index += 1;
            }
        }
        Ok(())
    } // list()

    // print a classic command line "Usage:" message
    // TODO: find out if there's a standard syntax for these
    pub fn print_usage(&self) {
        if let Some(invocation) = &self.invocation {
            eprintln!("Usage: {} [contents | link | list | update] [feed index] [entry index]", invocation)
        } else {
            eprintln!("Usage: rsst [contents | link | list | update] [feed index] [entry index]");
        }
    } // print_usage()

    // pull a specific feed to a local file in save_dir
    fn save_feed(&self, title: &str, url: &str) -> Result<(), RsstError> {
        // make sure a client was successfully created with Config::new()
        let client = match self.client.as_ref() {
            Some(client) => client,
            None => return Err(RsstError::ClientAbsent),
        };

        // essentially, get the plain text from this url. Error handling ensues
        let body = match client.get(url).send() {
            Ok(response) => match response.text() {
                Ok(string) => string,
                Err(_) => return Err(RsstError::ClientRequestEmpty),
            },
            Err(_) => return Err(RsstError::ClientRequestFail),
        };
        let path = format!("{}{}", self.save_dir, title);
        match fs::write(path, body) {
            Ok(_) => Ok(()),
            Err(e) => Err(RsstError::Io(e)),
        }
    }

    // use save_feed() to pull every feed
    // TODO: one day, consider making this async
    pub fn update(&self) -> Result<(), RsstError> {
        let parser = parser_new(&self.opml)?;
        let mut index = 0;
        // we can't use our usual parser_advance() tricks here because we need multiple attributes
        // from the same tag
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    // we only care about outline tags
                    if name.local_name != String::from("outline") {
                        continue;
                    }

                    let mut title = String::new();
                    let mut xml_url = String::new();

                    // get both xml_url and title attributes
                    for attribute in attributes {
                        if let "xmlUrl" = &attribute.name.local_name[..] {
                            xml_url = attribute.value.clone();
                        } else if let "title" = &attribute.name.local_name[..] {
                            title = attribute.value.clone();
                        }
                    }
                    
                    println!("Updating ({}) {}", index, title);

                    // if something goes wrong, let the user know, but keep updating
                    if let Err(RsstError::ClientRequestFail) = self.save_feed(&title, &xml_url) {
                        eprintln!("\tFailed updating {}", title);
                    }
                    index += 1;
                }
                Err(e) => return Err(RsstError::Xml(e)),
                _ => {}
            }
        }
        Ok(())
    } // update()

} // impl Config

// we use this struct for all errors in this program. It's a questionably good idea, but it keeps
// return types consistent and plays well with rust's '?' operator
#[derive(Debug)]
pub enum RsstError {
    // reqwest related errors
    ClientAbsent,
    ClientRequestEmpty,
    ClientRequestFail,
    // a function expected a Some(argument) in Config that was None
    IndexAbsent,
    // wrap io::Error for nice '?' usage
    Io(std::io::Error),
    // parser didn't find the tag/attribute we asked it to in parser_advance()
    ParserTagAbsent,
    ParserAttributeAbsent,
    // wrap xml::reader::Error for nice '?' usage
    Xml(xml::reader::Error),
}

// rss feeds contain dates in RFC 822 standard format. atom ... in ISO 8601 (the superior) format.
// take either and return mm-dd for printing
fn date_parse(date: &str) -> String {
    // I'm pretty sure RFC always contains a comma, and ISO never does, so this first block is RFC
    if date.contains(",") {
        // slice indexing depends on whether the date is a single digit because RFC has no padding
        // zero
        let single_digit =
            if &date[6..7] == " " {
                true
            } else {
                false
            }
        ;

        let month =
            if single_digit {
                &date[7..10]
            } else {
                &date[8..11]
            }
        ;

        // whose idea was the textual month in this standard
        let month_number = match month {
            "Jan" => "01", "Feb" => "02", "Mar" => "03",
            "Apr" => "04", "May" => "05", "Jun" => "06",
            "Jul" => "07", "Aug" => "08", "Sep" => "09",
            "Oct" => "10", "Nov" => "11", "Dec" => "12",
            _ => "00",
        };

        // introduce a padding zero if necessary
        if single_digit {
            format!("{}-0{}", month_number, &date[5..6])
        } else {
            format!("{}-{}", month_number, &date[5..7])
        }
    } else {
        // parse month-day from ISO 8601
        String::from(&date[5..10])
    }
} // date_parse()

// This function is hard to explain and probably not good logical practice, but its versatility
// brings me joy.
// Basically:
//      Take in a reference to an event-based parser. Iterate through events until we've seen
//      `count` instances of any of the tags specified in `tags`. Once we're there, if `attr` is
//      Some(string), attempt to get the value of the attribute of the tag we've landed on that
//      matches that string. Return an Option accordingly. Wrap everything in a Result because this
//      can go wrong.
fn parser_advance(parser: &mut EventReader<BufReader<File>>, tags: Vec<&str>, count: u8, attr: Option<&str>) -> Result<Option<String>, RsstError> {
    let mut count_current = 0;
    loop {
        match parser.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                // only act on tags specified in `tags`
                if tags.contains(&&name.local_name[..]) {
                    count_current += 1;
                    // if we've hit all of the tags that we wanted to, get the value of an
                    // attribute if requested
                    if count_current == count {
                        if let Some(string) = attr {
                            // search for an attribute with the name given in `attr`
                            for attribute in attributes {
                                if &attribute.name.local_name == string {
                                    return Ok(Some(attribute.value.clone()));
                                }
                            }
                        }
                        // in this block, we've iterated as much as we wanted to, so...
                        break;
                    }
                }
            }
            // upon prematurely hitting end of document, leave
            Ok(XmlEvent::EndDocument) => break,
            Err(e) => return Err(RsstError::Xml(e)),
            _ => {}
        }
    }
    Ok(None)
} // parser_advance()

// convenience function for instantiating parsers with proper error handling
fn parser_new(path: &str) -> Result<EventReader<BufReader<File>>, RsstError> {
    Ok(EventReader::new(
        BufReader::new(
            match File::open(path) {
                Ok(file) => file,
                Err(e) => return Err(RsstError::Io(e)),
            }
        )
    ))
} // parser_new()
