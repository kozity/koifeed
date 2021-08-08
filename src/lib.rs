#![warn(missing_docs)]

//! This crate provides two simple newtypes over `String`s as well as a few convenience functions
//! to ease the manipulation of newsfeeds in RSS 2.0 or Atom format.

use xml::reader::{Error, EventReader, XmlEvent};

const KEYS_CONTENT: [&str; 2] = ["content", "description"];
const KEYS_DATE:    [&str; 4] = ["published", "updated", "pubDate", "date"];
const KEYS_ENTRY:   [&str; 2] = ["entry", "item"];
const KEYS_TITLE:   [&str; 1] = ["title"];

/// A newtype struct to help manipulation of an OPML 2.0 subscription list.
pub struct Opml(String);

impl Opml {
    /// Constructor. Unlike `Feed`, `Opml` struct initialization is a one time cost, and OPML
    /// is generally relatively short, so this constructor checks the basic validity of the
    /// underlying XML.
    pub fn new(text: String) -> Result<Self, Error> {
        let parser = EventReader::new(text.as_bytes());
        let maybe_error = parser
            .into_iter()
            .find(|event| matches!(event, Err(Error { .. })));
        match maybe_error {
            Some(Err(err)) => Err(err),
            _ => Ok(Self(text)),
        }
    }

    /// Returns an iterator over attribute values for each `<outline>` element whose
    /// attribute name exactly matches that given. For simple cases such as the `text` or
    /// `xmlUrl` attributes, other convenience methods are provided.
    pub fn attribute_values(&self, search_attribute: &'static str) -> impl Iterator<Item = String> + '_ {
        let parser = EventReader::new(self.0.as_bytes());
        parser
            .into_iter()
            .skip_while(|event| {
                match event {
                    Ok(XmlEvent::StartElement { name, .. }) => {
                        name.local_name != "outline"
                    },
                    _ => true,
                }
            })
            .filter_map(move |event| {
                match event {
                    Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                        if name.local_name == "outline" {
                            for attribute in attributes {
                                if attribute.name.local_name == search_attribute {
                                    return Some(attribute.value);
                                }
                            }
                        }
                        None
                    },
                    _ => None,
                }
            })
    }

    /// Returns an iterator over attribute values for each `<outline>` element whose
    /// attribute name exactly matches that given. This works for optional attributes by leaving a
    /// `None` element in the returned iterator.
    pub fn attribute_values_optional(&self, search_attribute: &'static str) -> impl Iterator<Item = Option<String>> + '_ {
        let parser = EventReader::new(self.0.as_bytes());
        parser
            .into_iter()
            .skip_while(|event| {
                match event {
                    Ok(XmlEvent::StartElement { name, .. }) => {
                        name.local_name != "outline"
                    },
                    _ => true,
                }
            })
            .filter_map(move |event| {
                match event {
                    Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                        if name.local_name == "outline" {
                            for attribute in attributes {
                                if attribute.name.local_name == search_attribute {
                                    return Some(Some(attribute.value));
                                }
                            }
                            return Some(None);
                        }
                        None
                    },
                    _ => None,
                }
            })
    }

    /// Find the first OPML entry whose "text" attribute is a non-strict superstring of the given
    /// key; return the full value of that attribute if found.
    pub fn find(&self, key: &str) -> Option<String> {
        let titles = self.attribute_values("text");
        for title in titles {
            if title.contains(key) {
                return Some(title);
            }
        }
        None
    }

    /// Convenience function returning an iterator over optional HTML links given in the OPML.
    /// These are given as options because OPML 2.0 does not require the "htmlUrl" attribute.
    pub fn links_html(&self) -> impl Iterator<Item = Option<String>> + '_ {
        self.attribute_values_optional("htmlUrl")
    }

    /// Convenience function returning an iterator over XML feed links given in the OPML. The
    /// "xmlUrl" attribute is required by OPML subscription lists.
    pub fn links_xml(&self) -> impl Iterator<Item = String> + '_ {
        self.attribute_values("xmlUrl")
    }

    /// Convenience function returning an iterator over all tag lists of entries in the
    /// OPML.
    pub fn tags(&self) -> impl Iterator<Item = Vec<String>> + '_ {
        self.attribute_values_optional("category")
            .into_iter()
            .map(|tag_opt| {
                match tag_opt {
                    Some(tag_string) => {
                        tag_string
                            .split(',')
                            .map(String::from)
                            .collect()
                    },
                    None => Vec::new(),
                }
            })
    }

    /// Public accessor to this struct's underlying `String`.
    pub fn text(&self) -> &str { &self.0 }

    /// Convenience function returning an iterator over all titles of entries in the OPML.
    pub fn titles(&self) -> impl Iterator<Item = String> + '_ {
        self.attribute_values("text")
    }
}

/// A newtype struct to help manipulation of an RSS 2.0 or Atom feeds.
pub struct Feed(String);

impl Feed {
    
    /* SECTION: associated functions */

    /// A simple constructor. It performs no error checking on the XML string supplied to it.
    pub fn new(text: String) -> Self {
        Self(text)
    }

    /// Like `new()`, but checks the supplied XML for errors as designated by `xml-rs`. For
    /// long feeds, this may noticeably impact performance.
    pub fn new_check_xml(text: String) -> Result<Self, Error> {
        let parser = EventReader::new(text.as_bytes());
        let maybe_error = parser
            .into_iter()
            .find(|event| matches!(event, Err(Error { .. })));
        match maybe_error {
            Some(Err(err)) => Err(err),
            _ => Ok(Self(text)),
        }
    }

    /* SECTION: methods */

    /// Convenience function returning an iterator over the main contents/descriptions of all
    /// entries in the feed.
    pub fn contents(&self) -> impl Iterator<Item = String> + '_ {
        self.element_contents(&KEYS_CONTENT)
    }

    /// Convenience function returning an iterator over the dates of all entries in the feed,
    /// all given in ISO-8601 yyyy-mm-dd format.
    pub fn dates(&self) -> impl Iterator<Item = String> + '_ {
        self.element_contents(&KEYS_DATE)
            .map(|date_string| date_parse(&date_string))
    }

    /// Returns an iterator over the inner contents of all elements found whose names
    /// match one of those given in `element_names`. For simple cases such as date or main
    /// content elements, other convenience methods are provided.
    fn element_contents<'a>(&'a self, element_names: &'static [&str]) -> impl Iterator<Item = String> + 'a {
        let parser = EventReader::new(self.0.as_bytes());
        parser
            .into_iter()
            .scan((false, false), move |(awaiting_element, hit_element), event| {
                match event {
                    Ok(XmlEvent::StartElement { name, .. }) => {
                        let name = &name.local_name[..];
                        if KEYS_ENTRY.contains(&name) {
                            *awaiting_element = true;
                        } else if element_names.contains(&name) 
                            && *awaiting_element {
                            *hit_element = true;
                            *awaiting_element = false;
                        }
                        Some(None)
                    },
                    Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => {
                        if *hit_element {
                            *hit_element = false;
                            Some(Some(string))
                        } else {
                            Some(None)
                        }
                    },
                    _ => Some(None),
                }
            })
            .filter_map(|option| match option {
                Some(string) => Some(string),
                _ => None,
            })
    }

    /// Returns an iterator over all enclosure URL's from this feed.
    pub fn enclosure_links(&self) -> impl Iterator<Item = String> + '_ {
        let parser = EventReader::new(self.0.as_bytes());
        parser
            .into_iter()
            .filter_map(|event| {
                match event {
                    Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                        if name.local_name == "enclosure" {
                            attributes
                                .into_iter()
                                .find(|attribute| attribute.name.local_name == "url")
                        } else {
                            None
                        }
                    },
                    _ => None,
                }
            })
            .map(|attribute| attribute.value)
    }

    /// Convenience function returning an iterator over all entry links from the feed. Note that
    /// this method ignores enclosures.
    pub fn links(&self) -> impl Iterator<Item = String> + '_ {
        let parser = EventReader::new(self.0.as_bytes());
        parser
            .into_iter()
            .scan((false, false), move |(awaiting_link, hit_link), event| {
                match event {
                    Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                        if KEYS_ENTRY.contains(&&name.local_name[..]) {
                            *awaiting_link = true;
                        } else if name.local_name == "link" && *awaiting_link {
                            *awaiting_link = false;
                            let maybe_href_attr = attributes
                                .into_iter()
                                .find(|attribute| attribute.name.local_name == "href");
                            if let Some(href_attr) = maybe_href_attr {
                                *hit_link = false;
                                return Some(Some(href_attr.value));
                            } else {
                                *hit_link = true;
                                return Some(None);
                            }
                        }
                        Some(None)
                    },
                    Ok(XmlEvent::Characters(string)) | Ok(XmlEvent::CData(string)) => {
                        if *hit_link {
                            *hit_link = false;
                            Some(Some(string))
                        } else {
                            Some(None)
                        }
                    },
                    _ => Some(None),
                }
            })
            .filter_map(|option| match option {
                Some(string) => Some(string),
                _ => None,
            })
    }

    /// Convenience function returning an iterator of all entry titles in the feed.
    pub fn titles(&self) -> impl Iterator<Item = String> + '_ {
        self.element_contents(&KEYS_TITLE)
    }

    /// Public accessor to this struct's underlying `String`.
    pub fn text(&self) -> &str { &self.0 }
}

/// Naively but (probably) correctly converts the RFC 822 date format into ISO-8601.
pub fn date_parse(date: &str) -> String {
    if date.contains(',') {
        let single_digit = &date[6..7] == " ";
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
            format!("{}-{}-0{}", &date[11..15], month_number, &date[5..6])
        } else {
            format!("{}-{}-{}",  &date[12..16], month_number, &date[5..7])
        }
    } else {
        String::from(&date[..10])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn date_from_iso() {
        let iso_attempt = date_parse("2003-12-13T18:30:02-05:00");
        let iso_good    = String::from("2003-12-13");
        assert_eq!(iso_attempt, iso_good);
    }

    #[test]
    fn date_from_rfc_one_digit() {
        let rfc_attempt = date_parse("Sun, 9 May 2002 15:21:36 GMT");
        let rfc_good    = String::from("2002-05-09");
        assert_eq!(rfc_attempt, rfc_good);
    }

    #[test]
    fn date_from_rfc_two_digit() {
        let rfc_attempt = date_parse("Sun, 19 May 2002 15:21:36 GMT");
        let rfc_good    = String::from("2002-05-19");
        assert_eq!(rfc_attempt, rfc_good);
    }

    /* SECTION: OPML */

    static opml: &str = r#"
        <?xml version="1.0" encoding="utf-8"?> <opml version="2.0">
            <head />
            <body>
                <outline
                    category="software"
                    text="archlinux"
                    type="rss"
                    htmlUrl="https://archlinux.org"
                    xmlUrl="https://archlinux.org/feeds/news/"
                />
                <outline
                    category="audio,software"
                    text="buildingwithrust"
                    type="rss"
                    htmlUrl="https://seanchen1991.github.io"
                    xmlUrl="https://anchor.fm/s/4928bbdc/podcast/rss"
                />
                <outline
                    category="video,leisure,education"
                    text="kurzgesagt"
                    type="rss"
                    htmlUrl="https://www.youtube.com/channel/UCsXVk37bltHxD1rDPwtNM8Q"
                    xmlUrl="https://www.youtube.com/feeds/videos.xml?channel_id=UCsXVk37bltHxD1rDPwtNM8Q"
                />
                <outline
                    category="software"
                    text="neovim"
                    type="rss"
                    htmlUrl="https://neovim.io"
                    xmlUrl="https://neovim.io/news.xml"
                />
                <outline
                    category="news"
                    text="npr"
                    type="rss"
                    htmlUrl="https://www.npr.org/"
                    xmlUrl="https://feeds.npr.org/1001/rss.xml"
                />
                <outline
                    text="nprupfirst"
                    type="rss"
                    htmlUrl="https://www.npr.org/podcasts/510318/up-first"
                    xmlUrl="https://feeds.npr.org/510318/podcast.xml"
                />
                <outline
                    category="news"
                    text="propublica"
                    type="rss"
                    htmlUrl="https://www.propublica.org/"
                    xmlUrl="http://feeds.propublica.org/propublica/main"
                />
                <outline
                    category="news,software"
                    text="slashdot"
                    type="rss"
                    htmlUrl="https://slashdot.org/"
                    xmlUrl="http://rss.slashdot.org/Slashdot/slashdotMain"
                />
                <outline
                    category="blog"
                    text="tykozic.net"
                    type="rss"
                    htmlUrl="http://tykozic.net/"
                    xmlUrl="http://tykozic.net/atom.xml"
                />
            </body>
        </opml>
    "#;

    #[test]
    fn opml_attribute_values() {
        let opml_struct = Opml::new(opml.to_string()).unwrap();
        let actual: Vec<String> = opml_struct
            .attribute_values("text")
            .collect();
        let expected = vec![
            String::from("archlinux"),
            String::from("buildingwithrust"),
            String::from("kurzgesagt"),
            String::from("neovim"),
            String::from("npr"),
            String::from("nprupfirst"),
            String::from("propublica"),
            String::from("slashdot"),
            String::from("tykozic.net"),
        ];
        assert_eq!(actual, expected);
    }

    /* SECTION: Feed */

    static atom: &str = r#"
        <?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom">

            <title>Example Feed</title>
            <link href="http://example.org/"/>
            <updated>2003-12-13T18:30:02Z</updated>
            <author>
                <name>John Doe</name>
            </author>
            <id>urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6</id>

            <entry>
                <title>Atom-Powered Robots Run Amok</title>
                <link href="http://example.org/2003/12/13/atom03"/>
                <id>urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a</id>
                <updated>2003-12-13T18:30:02Z</updated>
                <summary>Some text.</summary>
            </entry>

            <entry>
                <title>Ty's homegrown sample entry</title>
                <link href="http://tykozic.net" />
                <id>http://tykozic.net/posts/rss-part-1</id>
                <updated>2021-08-06T15:32:35-05:00</updated>
                <summary>yee</summary>
            </entry>
        
        </feed>
    "#;

    // TODO
    /*
    #[test]
    fn rss_element_contents() {
        let feed = Feed::new(rss.to_string()).unwrap();
        let actual = feed.element_contents(&KEYS_DATE);
        let expected = vec![
        ];
        assert_eq!(actual, expected);
    }
    */

    #[test]
    fn atom_element_contents() {
        let feed = Feed::new(atom.to_string()).unwrap();
        let actual: Vec<String> = feed
            .element_contents(&KEYS_DATE)
            .collect();
        let expected = vec![
            String::from("2003-12-13T18:30:02Z"),
            String::from("2021-08-06T15:32:35-05:00"),
        ];
        assert_eq!(actual, expected);
    }
}
