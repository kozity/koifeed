use reqwest::blocking::Client;
use std::env;
use std::fs::{self, File};
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};

fn main() -> Result<(), RsstError> {
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
    pub opml: String,
    pub save_dir: String,
    pub invocation: Option<String>,
    pub subcommand: Option<String>,
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

    pub fn content(&self) -> Result<(), Error> {
        if let None = self.index_feed {
            return Err(RsstError::IndexAbsent);
        }
        if let None = self.index_entry {
            return Err(RsstError::IndexAbsent);
        }
        let mut parser = parser_new(&self.opml)?;
        let mut title = String::new();
        if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("title"))? {
            title = string;
        } else {
            println!("Error: feed not found.");
        }
        let mut parser = parser_new(&format!("{}{}", self.save_dir, title))?;
        parser_advance(&mut parser, vec!("entry", "item"), self.index_entry.unwrap() + 1, None)?;
        parser_advance(&mut parser, vec!("content", "description"), 1, None)?;
        match &parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) =>
                println!("{}", string),
            _ => eprintln!("Error: contents not found."),
        }
        Ok(())
    } // content()

    fn date_get_feed(&self, feed_name: &str) -> Result<String, Error> {
        let mut parser = parser_new(&format!("{}{}", self.save_dir, feed_name))?;
        let mut date = String::new();
        parser_advance(&mut parser, vec!("entry", "item"), 1, None)?;
        parser_advance(&mut parser, vec!("published", "date", "pubDate"), 1, None)?;
        match &parser.next() {
            Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => date = String::from(string),
            _ => eprintln!("Error: date not found."),
        }
        Ok(date_parse(&date))
    } // date_get_feed()

    pub fn link(&self) -> Result<(), Error> {
        if let None = self.index_feed {
            println!("Error: no index argument(s) found.");
            return Ok(());
        }
        let mut parser = parser_new(&self.opml)?;
        let mut title = String::new();
        if let None = self.index_entry {
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("htmlUrl"))? {
                println!("{}", string);
            } else {
                println!("Error: link not found.");
            }
            return Ok(());
        } else {
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), self.index_feed.unwrap() + 1, Some("title"))? {
                title = string;
            } else {
                println!("Error: feed not found.");
            }
        }
        let mut parser = parser_new(&format!("{}{}", self.save_dir, title))?;
        parser_advance(&mut parser, vec!("entry", "item"), self.index_entry.unwrap() + 1, None)?;
        if let Some(string) = parser_advance(&mut parser, vec!("link"), 1, Some("href"))? {
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

    pub fn list(&self) -> Result<(), Error> {
        if let Some(index_feed) = self.index_feed { // list from individual feed
            let mut parser = parser_new(&self.opml)?;
            let mut title = String::new();
            if let Some(string) = parser_advance(&mut parser, vec!("outline"), index_feed + 1, Some("title"))? {
                title = string;
            }
            let mut parser = parser_new(&format!("{}{}", self.save_dir, title))?;

            println!("({}) {}", index_feed, title);
            println!("Entry\tDate\tTitle");

            let mut date: Option<String> = None;
            let mut title: Option<String> = None;
            let mut index = 0;

            // skip header content until entries
            loop {
                parser_advance(&mut parser, vec!("entry", "item"), 1, None)?;
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
                if let (Some(title_inner), Some(date_inner)) = (&title, &date) {
                    println!("{}\t{}\t{}", index, date_inner, title_inner);
                    date = None;
                    title = None;
                    index += 1;
                }
            }
        } else { // list all feeds
            let mut parser = parser_new(&self.opml)?;
            let mut index = 0;
            println!("Feed\tUpdated\tTitle");
            loop {
                let title;
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

    pub fn print_usage(&self) {
        if let Some(invocation) = &self.invocation {
            eprintln!("Usage: {} [contents | link | list | update] [feed index] [entry index]", invocation)
        } else {
            eprintln!("Usage: rsst [contents | link | list | update] [feed index] [entry index]");
        }
    }

    fn save_feed(&self, title: &str, url: &str) -> Result<(), Error> {
        let client = match self.client.as_ref() {
            Some(client) => client,
            None => return Err(RsstError::ClientAbsent),
        };
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

    // TODO: one day, consider making this async
    pub fn update(&self) -> Result<(), Error> {
        let parser = parser_new(&self.opml)?;
        let mut index = 0;
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    if name.local_name != String::from("outline") {
                        continue;
                    }
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

#[derive(Debug)]
pub enum RsstError {
    ClientAbsent,
    ClientRequestEmpty,
    ClientRequestFail,
    IndexAbsent,
    Io(std::io::Error),
    ParserTagAbsent,
    ParserAttributeAbsent,
    Xml(xml::reader::Error),
}

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

fn parser_advance(parser: &mut EventReader<BufReader<File>>, tags: Vec<&str>, count: u8, attr: Option<&str>) -> Result<Option<String>, Error> {
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
                                    return Ok(Some(attribute.value.clone()));
                                }
                            }
                        }
                        break;
                    }
                }
            }
            Ok(XmlEvent::EndDocument) => break,
            Err(e) => return Err(RsstError::Xml(e)),
            _ => {}
        }
    }
    Ok(None)
} // parser_advance()

fn parser_new(path: &str) -> Result<EventReader<BufReader<File>>, Error> {
    Ok(EventReader::new(
        BufReader::new(
            match File::open(path) {
                Ok(file) => file,
                Err(e) => return Err(RsstError::Io(e)),
            }
        )
    ))
}
