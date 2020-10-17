// TODO: implement an ErrorKind struct for better error handling
use reqwest::blocking::Client;
use std::env;
use std::fs::{self, File};
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};

pub struct Config {
    pub opml: String,
    pub save_dir: String,
    pub invocation: Option<String>,
    pub subcommand: Option<String>,
    pub client: Option<Client>,
    pub index_feed: Option<u8>,
    pub index_entry: Option<u8>,
}
impl Config {
    //TODO: validate indices
    pub fn new(opml: String, save_dir: String) -> Self {
        let mut args = env::args();
        let invocation = args.next();
        let subcommand = args.next();
        let index_feed_string = args.next();
        let index_entry_string = args.next();
        let mut index_feed = None;
        let mut index_entry = None;
        let mut client = None;

        let save_dir_suffixed =
            if save_dir.ends_with('/') {
                save_dir
            } else {
                format!("{}/", save_dir)
            }
        ;

        if let Some(sub) = &subcommand {
            if &sub[..] == "update" {
                client = Some(Client::new());
            }
        }

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

    pub fn content(&self) -> Result<(), std::io::Error> {
        if let None = self.index_entry {
            eprintln!("Error: no entry index argument found.");
            return Ok(());
        }
        let mut parser = EventReader::new(
            BufReader::new(
                File::open(&self.opml)?
            )
        );
        let mut title = String::new();
        if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("title")) {
            title = string;
        } else {
            println!("Error: feed not found.");
        }
        let mut parser = EventReader::new(
            BufReader::new(
                File::open(
                    format!("{}{}", self.save_dir, title)
                )?
            )
        );
        parser_advance(&mut parser, vec!("entry", "item"), self.index_feed.unwrap() + 1, None);
        parser_advance(&mut parser, vec!("content", "description"), 1, None);
        match &parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) =>
                println!("{}", string),
            _ => eprintln!("Error: contents not found."),
        }
        Ok(())
    } // content()

    fn date_get_feed(&self, feed_name: &str) -> Result<String, std::io::Error> {
        let mut date = String::new();
        let mut parser = EventReader::new(
            BufReader::new(
                File::open(
                    format!("{}{}", self.save_dir, feed_name)
                )?
            )
        );
        parser_advance(&mut parser, vec!("entry", "item"), 1, None);
        parser_advance(&mut parser, vec!("published", "date", "pubDate"), 1, None);
        match &parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => date = String::from(string),
            _ => eprintln!("Error: date not found."),
        }
        Ok(date_parse(&date))
    } // date_get_feed()

    pub fn link(&self) -> Result<(), std::io::Error> {
        if let None = self.index_feed {
            println!("Error: no index argument(s) found.");
            return Ok(());
        }
        let mut parser = EventReader::new(
            BufReader::new(
                File::open(&self.opml)?
            )
        );
        let mut title = String::new();
        if let None = self.index_entry {
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("htmlUrl")) {
                println!("{}", string);
            } else {
                println!("Error: link not found.");
            }
            return Ok(());
        } else {
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("title")) {
                title = string;
            } else {
                println!("Error: feed not found.");
            }
        }
        let mut parser = EventReader::new(
            BufReader::new(
                File::open(
                    format!("{}{}", self.save_dir, title)
                )?
            )
        );
        parser_advance(&mut parser, vec!("entry", "item"), self.index_entry.unwrap() + 1, None);
        if let Some(string) = parser_advance(&mut parser, vec!("link"), 1, Some("href")) {
            println!("{}", string);
        } else {
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
    }

    // TODO: this is real slow right now for some feeds
    pub fn list(&self) -> Result<(), std::io::Error> {
        if let Some(index_feed) = self.index_feed { // list from individual feed
            let mut parser = EventReader::new(
                BufReader::new(
                    File::open(&self.opml)?
                )
            );
            let mut title = String::new();
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), index_feed + 1, Some("title")) {
                title = string;
            }
            let mut parser = EventReader::new(
                BufReader::new(
                    File::open(
                        format!("{}{}", self.save_dir, title)
                    )?
                )
            );

            println!("({}) {}", index_feed, title);
            println!("Entry\tDate\tTitle");

            let mut date: Option<String> = None;
            let mut title: Option<String> = None;
            let mut index = 0;

            // skip header content until entries
            parser_advance(&mut parser, vec!("entry", "item"), 1, None);
            loop {
                parser_advance(&mut parser, vec!("title"), 1, None);
                match &parser.next() {
                    Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => title = Some(string.clone()),
                    Ok(XmlEvent::EndDocument) => break,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                    _ => {}
                }
                parser_advance(&mut parser, vec!("published", "date", "pubDate"), 1, None);
                match &parser.next() {
                    Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => date = Some(date_parse(string)),
                    Ok(XmlEvent::EndDocument) => break,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                    _ => {}
                }
                if let (Some(title_inner), Some(date_inner)) = (&title, &date) {
                    println!("{}\t{}\t{}", index, date_inner, title_inner);
                    date = None;
                    title = None;
                    index += 1;
                }
            }
        } else { // list all feeds
            let mut parser = EventReader::new(
                BufReader::new(
                    File::open(&self.opml)?
                )
            );
            let mut index = 0;
            println!("Feed\tUpdated\tTitle");
            loop {
                let title;
                if let Some(string) = parser_advance(&mut parser, vec!("outline"), 1, Some("title")) {
                    title = string;
                } else {
                    break;
                }
                println!("{}\t{}\t{}", index, self.date_get_feed(&title)?, title);
                index += 1;
            }
        }
        Ok(())
    } // list()

    pub fn print_usage(&self) {
        if let Some(invocation) = &self.invocation {
            eprintln!("Usage: {} [ contents | link | list | update ]", invocation)
        } else {
            eprintln!("Usage: rsst [ contents | link | list | update ]");
        }
    }

    fn save_feed(&self, title: &str, url: &str) -> Result<(), std::io::Error> {
        let body = self.client.as_ref().unwrap().get(url).send().expect("HTTP error").text().unwrap();
        let path = format!("{}{}", self.save_dir, title);
        fs::write(path, body)?;
        Ok(())
    }

    // TODO: one day, consider making this async
    pub fn update(&self) -> Result<(), std::io::Error> {
        let parser = EventReader::new(
            BufReader::new(
                File::open(&self.opml)?
            )
        );
        let mut index = 0;
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    if name.local_name != String::from("outline") {
                        continue;
                    }
                    index += 1;
                    let mut title = String::new();
                    let mut xml_url = String::new();
                    for attribute in attributes {
                        if let "xmlUrl" = &attribute.name.local_name[..] {
                            xml_url = attribute.value.clone();
                        } else if let "title" = &attribute.name.local_name[..] {
                            title = attribute.value.clone();
                        }
                    }
                    println!("Updating ({}) {}", index, title);
                    self.save_feed(&title, &xml_url)?;
                }
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        Ok(())
    } // update()

} // impl Config

fn date_parse(date: &str) -> String {
    if date.contains(",") { // parse month-day from RFC 822
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
    } else { // parse month-day from ISO 8601
        String::from(&date[5..10])
    }
} // date_parse()

fn parser_advance(parser: &mut EventReader<BufReader<File>>, tags: Vec<&str>, count: u8, attr: Option<&str>) -> Option<String> {
    let mut count_current = 0;
    loop {
        match parser.next() {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                if tags.contains(&&name.local_name[..]) {
                    count_current += 1;
                    if count_current == count {
                        if let Some(string) = attr {
                            for attribute in attributes {
                                if &attribute.name.local_name == string {
                                    return Some(attribute.value.clone());
                                }
                            }
                        }
                        break;
                    }
                }
            }
            Ok(XmlEvent::EndDocument) => {
                break;
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }
    None
} // parser_advance()
