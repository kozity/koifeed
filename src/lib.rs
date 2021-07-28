use reqwest::blocking::Client;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader};
use xml::reader::{EventReader, XmlEvent};

const KEYS_CONTENT: &[&str] = &["content", "description"];
const KEYS_DATE:    &[&str] = &["published", "date", "pubDate"];
const KEYS_ENTRY:   &[&str] = &["entry", "item"];
const KEYS_OUTLINE: &[&str] = &["outline"];

pub struct Config {
    pub entry_index: Option<usize>,
    pub feed_title:  Option<String>,
    pub invocation:  String,
    pub path_cache:  String,
    pub path_opml:   String,
    pub subcommand:  Option<String>,
}

impl Config {
    pub fn init() -> Result<Self, Error> {
        let path_home = match env::var("HOME") {
            Ok(string) => string,
            Err(_) => return Err(Error::EnvNoHome),
        };
        let path_cache = format!("{}/.local/share/rss", path_home);
        let path_opml = format!("{}/.config/feeds.opml", path_home);
        let mut args = env::args();
        let invocation = args.next().expect("absent invocation argument");
        let subcommand = args.next();
        let feed_title = match args.next() {
            Some(key) => Some(Self::feed_title_by_key(&key, &path_opml)?),
            None => None,
        };
        let entry_index = match args.next() {
            Some(string) => match string.parse::<usize>() {
                Ok(num) => Some(num),
                Err(_) => None,
            },
            None => None,
        };
        Ok(Self {
            entry_index,
            feed_title,
            invocation,
            path_cache,
            path_opml,
            subcommand,
        })
    }

    pub fn content(&self) -> Result<(), Error> {
        let entry_index = self.entry_index.unwrap();
        let feed_title = self.feed_title.as_ref().unwrap();
        let mut parser = parser_init(&format!("{}/{}", self.path_cache, feed_title))?;
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

    fn feed_title_by_key(feed_key: &str, path_opml: &str) -> Result<String, Error> {
        let mut parser = parser_init(path_opml)?;
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

    pub fn link_entry(&self) -> Result<(), Error> {
        let entry_index = self.entry_index.unwrap();
        let feed_title = self.feed_title.as_ref().unwrap();
        let mut parser = parser_init(&format!("{}/{}", self.path_cache, feed_title))?;
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

    pub fn link_feed(&self) -> Result<(), Error> {
        let mut parser = parser_init(&self.path_opml)?;
        loop {
            match parser.next() {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    if name.local_name == "outline" {
                        match attributes.iter().find(|attribute| attribute.name.local_name == "title") {
                            Some(attr) => {
                                if &attr.value == self.feed_title.as_ref().unwrap() {
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

    pub fn list_all(&self) -> Result<(), Error> {
        let mut parser = parser_init(&self.path_opml)?;
        println!("DATE\tTITLE");
        loop {
            let title = match parser_advance(&mut parser, KEYS_OUTLINE, 1, Some("title")) {
                Ok(string) => string,
                Err(Error::XmlUnexpectedEOF) => break,
                Err(e) => return Err(Error::from(e)),
            };
            // Using the title, retrieve the date from the feed file.
            let mut feed_parser = parser_init(&format!("{}/{}", self.path_cache, title))?;
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
    pub fn list_feed(&self) -> Result<(), Error> {
        let feed_title = self.feed_title.as_ref().unwrap();
        let mut index = 0;
        let mut parser = parser_init(&format!("{}/{}", self.path_cache, feed_title))?;
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

    // Arguments are error-checked here; unwrap() may be used within these constituent functions.
    pub fn run(&self) -> Result<(), Error> {
        match self.subcommand.as_deref() {
            Some("content") => match (&self.feed_title, &self.entry_index) {
                (Some(_), Some(_)) => self.content(),
                _ => Err(Error::ArgsNoEntry),
            },
            Some("link") => match (&self.feed_title, &self.entry_index) {
                (Some(_), Some(_)) => self.link_entry(),
                (Some(_), None) => self.link_feed(),
                _ => Err(Error::ArgsNoFeed),
            },
            Some("list") => match &self.feed_title {
                Some(_) => self.list_feed(),
                None => self.list_all(),
            },
            Some("update") => self.update(),
            _ => Err(Error::ArgsBadSubcommand),
        }
    }

    pub fn update(&self) -> Result<(), Error> {
        let client = Client::new();
        // We can't use our usual parser_advance() tricks here because we need multiple attributes from the same tag.
        for e in parser_init(&self.path_opml)? {
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
                    fs::write(&format!("{}/{}", self.path_cache, title), body)?;
                }
                Err(e) => return Err(Error::from(e)),
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    ArgsBadSubcommand,
    ArgsNoEntry,
    ArgsNoFeed,
    BadContent,
    BadEnclosure,
    EnvNoHome,
    Io(io::Error),
    KeyNoMatch,
    ParserAttributeAbsent,
    ParserDateAbsent,
    Xml(xml::reader::Error),
    XmlUnexpectedEOF,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Self::Io(e) }
}

impl From<xml::reader::Error> for Error {
    fn from(e: xml::reader::Error) -> Self { Self::Xml(e) }
}

pub fn date_parse(date: &str) -> String {
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

pub fn parser_advance(parser: &mut EventReader<BufReader<File>>, tags: &[&str], count: usize, attr: Option<&str>) -> Result<String, Error> {
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

pub fn parser_init(path: &str) -> Result<EventReader<BufReader<File>>, Error> {
    Ok(EventReader::new(BufReader::new(File::open(path)?)))
}
