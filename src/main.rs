use reqwest::blocking::Client;
use std::fs::{self, File};
use std::io::{self, BufReader};
use xml::reader::{EventReader, XmlEvent};

macro_rules! path_to {
    ($t:expr) => { &format!("{}/{}", PATH_CACHE, $t) };
}

#[derive(Debug)]
pub enum Error {
    ArgsBadSubcommand,
    ArgsEntryParse,
    ArgsNoEntry,
    ArgsNoFeed,
    BadContent,
    BadEnclosure,
    Io(io::Error),
    KeyNoMatch,
    ParserTagAbsent,
    ParserAttributeAbsent,
    ParserDateAbsent,
    UnspecifiedEntry,
    UnspecifiedFeed,
    Xml(xml::reader::Error),
    XmlUnexpectedEOF
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Self::Io(e) }
}
impl From<xml::reader::Error> for Error {
    fn from(e: xml::reader::Error) -> Self { Self::Xml(e) }
}

const KEYS_CONTENT: &[&str] = &["content", "description"];
const KEYS_DATE:    &[&str] = &["published", "date", "pubDate"];
const KEYS_ENTRY:   &[&str] = &["entry", "item"];
const KEYS_OUTLINE: &[&str] = &["outline"];
const PATH_CACHE:   &str    = "/home/ty/.local/share/rss";
const PATH_OPML:    &str    = "/home/ty/.config/feeds.opml";

fn main() -> Result<(), Error> {
    let mut args = std::env::args();
    //let invocation = args.next().expect("absent invocation argument");
    args.next(); // eat invocation
    let subcommand = args.next();
    let feed_title = match args.next() {
        Some(key) => Some(feed_title_by_key(&key)?),
        None => None,
    };
    let entry_index = match args.next() {
        Some(string) => match string.parse::<usize>() {
            Ok(num) => Some(num),
            Err(_) => return Err(Error::ArgsEntryParse),
        },
        None => None,
    };
    match subcommand.as_deref() {
        Some("content") => match (feed_title, entry_index) {
            (Some(f), Some(e)) => content(&f, e),
            _ => Err(Error::ArgsNoEntry),
        },
        Some("link") => match (feed_title, entry_index) {
            (Some(f), Some(e)) => link_entry(&f, e),
            (Some(f), None) => link_feed(&f),
            _ => Err(Error::ArgsNoFeed),
        },
        Some("list") => match feed_title {
            Some(f) => list_feed(&f),
            None => list_all(),
        },
        Some("update") => update(),
        _ => Err(Error::ArgsBadSubcommand),
    }
}

fn content(feed_title: &str, entry_index: usize) -> Result<(), Error> {
    let mut parser = parser_init(path_to!(feed_title))?;
    parser_advance(&mut parser, KEYS_ENTRY, entry_index + 1, None)?;
    parser_advance(&mut parser, KEYS_CONTENT, 1, None)?;
    match parser.next() {
        Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => {
            println!("{}", string);
            Ok(())
        },
        _ => Err(Error::BadContent),
    }
}

fn date_parse(date: &str) -> String {
    if date.contains(",") {
        let single_digit =
            if &date[6..7] == " " {
                true
            } else {
                false
            };
        let month =
            if single_digit {
                &date[7..10]
            } else {
                &date[8..11]
            };
        let month_number = match month {
            "Jan" => "01", "Feb" => "02", "Mar" => "03",
            "Apr" => "04", "May" => "05", "Jun" => "06",
            "Jul" => "07", "Aug" => "08", "Sep" => "09",
            "Oct" => "10", "Nov" => "11", "Dec" => "12",
            _ => "00",
        };
        if single_digit {
            format!("{}-0{}", month_number, &date[5..6])
        } else {
            format!("{}-{}", month_number, &date[5..7])
        }
    } else {
        String::from(&date[5..10])
    }
}

fn feed_title_by_key(feed_key: &str) -> Result<String, Error> {
    let mut parser = parser_init(PATH_OPML)?;
    loop {
        let title = match parser_advance(&mut parser, KEYS_OUTLINE, 1, Some("title")) {
            Ok(string) => string,
            Err(Error::XmlUnexpectedEOF) => break,
            Err(e) => return Err(e),
        };
        if title.contains(feed_key) { return Ok(title); }
    }
    Err(Error::KeyNoMatch)
}

fn link_entry(feed_title: &str, entry_index: usize) -> Result<(), Error> {
    let mut parser = parser_init(path_to!(feed_title))?;
    parser_advance(&mut parser, KEYS_ENTRY, entry_index + 1, None)?;
    let mut link = String::new();
    loop {
        match parser.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                if &name.local_name == "enclosure" {
                    match attributes.iter().find(|attribute| attribute.name.local_name == "url") {
                        Some(attr) => {
                            link = attr.value.clone();
                            break;
                        },
                        None => return Err(Error::BadEnclosure),
                    }
                } else if &name.local_name == "link" {
                    if link.is_empty() {
                        link = match attributes.iter().find(|attribute| attribute.name.local_name == "href") {
                            Some(attr) => attr.value.clone(),
                            None => match parser.next() {
                                Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => string,
                                Err(e) => return Err(Error::from(e)),
                                _ => continue,
                            },
                        };
                    } else {
                        break;
                    }
                }
            },
            Ok(XmlEvent::EndDocument) => {
                if link.is_empty() {
                    return Err(Error::XmlUnexpectedEOF);
                } else {
                    break;
                }
            },
            Err(e) => return Err(Error::from(e)),
            _ => {},
        }
    }
    println!("{}", link);
    Ok(())
}

fn link_feed(feed_title: &str) -> Result<(), Error> {
    let mut parser = parser_init(PATH_OPML)?;
    loop {
        match parser.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                if name.local_name == "outline" {
                    match attributes.iter().find(|attribute| attribute.name.local_name == "title") {
                        Some(attr) => {
                            if attr.value == feed_title {
                                match attributes.iter().find(|attribute| attribute.name.local_name == "htmlUrl") {
                                    Some(attr) => {
                                        println!("{}", attr.value);
                                        break;
                                    },
                                    None => return Err(Error::ParserAttributeAbsent),
                                }
                            }
                        },
                        None => return Err(Error::KeyNoMatch),
                    }
                }
            },
            Ok(XmlEvent::EndDocument) => return Err(Error::XmlUnexpectedEOF),
            Err(e) => return Err(Error::from(e)),
            _ => {},
        }
    }
    Ok(())
}

fn list_all() -> Result<(), Error> {
    let mut parser = parser_init(PATH_OPML)?;
    println!("DATE\tTITLE");
    loop {
        let title = match parser_advance(&mut parser, KEYS_OUTLINE, 1, Some("title")) {
            Ok(string) => string,
            Err(Error::XmlUnexpectedEOF) => break,
            Err(e) => return Err(Error::from(e)),
        };
        // Using the title, retrieve the date from the feed file.
        let mut feed_parser = parser_init(path_to!(title))?;
        parser_advance(&mut feed_parser, KEYS_ENTRY, 1, None)?;
        parser_advance(&mut feed_parser, KEYS_DATE, 1, None)?;
        let date = match feed_parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => date_parse(&string),
            _ => return Err(Error::ParserDateAbsent),
        };
        println!("{}\t{}", date, title);
    }
    Ok(())
}

// The current logic of this function is subject to assumed ordering of title and date elements.
fn list_feed(feed_title: &str) -> Result<(), Error> {
    let mut index = 0;
    let mut parser = parser_init(path_to!(feed_title))?;
    print!("FEED:\t{}\nINDEX\tDATE\tTITLE\n", feed_title);
    loop {
        match parser_advance(&mut parser, KEYS_ENTRY, 1, None) {
            Err(Error::XmlUnexpectedEOF) => break,
            Err(e) => return Err(e),
            _ => {},
        }
        parser_advance(&mut parser, &["title"], 1, None)?;
        let entry_title = match parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => string,
            Ok(XmlEvent::EndDocument) => break,
            Err(e) => return Err(Error::from(e)),
            _ => String::from(""),
        };
        parser_advance(&mut parser, KEYS_DATE, 1, None)?;
        let entry_date = match parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => date_parse(&string),
            Ok(XmlEvent::EndDocument) => break,
            Err(e) => return Err(Error::from(e)),
            _ => String::from(""),
        };
        print!("{}\t{}\t{}\n", index, entry_date, entry_title.trim_end());
        index += 1;
    }
    Ok(())
}

fn parser_advance(parser: &mut EventReader<BufReader<File>>, tags: &[&str], count: usize, attr: Option<&str>) -> Result<String, Error> {
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
                        match attr {
                            Some(string) => {
                                // search for an attribute with the name given in `attr`
                                for attribute in attributes {
                                    if &attribute.name.local_name == string {
                                        return Ok(attribute.value.clone());
                                    }
                                }
                            },
                            None => return Ok(String::new()),
                        }
                        // in this block, we've iterated as much as we wanted to, so...
                        break;
                    }
                }
            }
            Ok(XmlEvent::EndDocument) => return Err(Error::XmlUnexpectedEOF),
            Err(e) => return Err(Error::from(e)),
            _ => {}
        }
    }
    Err(Error::ParserAttributeAbsent)
}

fn parser_init(path: &str) -> Result<EventReader<BufReader<File>>, Error> {
    Ok(EventReader::new(BufReader::new(File::open(path)?)))
}

fn update() -> Result<(), Error> {
    let client = Client::new();
    // We can't use our usual parser_advance() tricks here because we need multiple attributes from the same tag.
    for e in parser_init(PATH_OPML)? {
        match e {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                if &name.local_name != "outline" { continue; }
                let mut title = String::new();
                let mut xml_url = String::new();
                // get both xml_url and title attributes
                for attribute in attributes {
                    if "xmlUrl" == &attribute.name.local_name[..] {
                        xml_url = attribute.value.clone();
                    } else if "title" == &attribute.name.local_name[..] {
                        title = attribute.value.clone();
                    }
                }
                println!("updating {}", title);
                // Essentially, get the plain text from this url. Error handling ensues.
                let body = match client.get(&xml_url).send() {
                    Ok(response) => match response.text() {
                        Ok(string) => string,
                        Err(_) => {
                            eprintln!("\terror: empty response from {}", xml_url);
                            continue;
                        },
                    },
                    Err(_) => {
                        eprintln!("\terror: request failure for {}", xml_url);
                        continue;
                    },
                };
                fs::write(path_to!(title), body)?;
            }
            Err(e) => return Err(Error::from(e)),
            _ => {}
        }
    }
    Ok(())
}
