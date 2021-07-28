use rsst::{Config, Error};
use std::process;

fn main() -> () {
    match Config::init() {
        Ok(c) => {
            match c.run() {
                Ok(()) => process::exit(0),
                Err(Error::ArgsBadSubcommand) => {
                    eprint!("usage: {} (content | link | list | update) [subcommand arguments]\n", c.invocation);
                    eprint!("subcommand arguments:\n");
                    eprint!("\tcontent:\tfeed_key entry_index\n");
                    eprint!("\t   link:\tfeed_key [entry_index]\n");
                    eprint!("\t   list:\t[feed_key]\n");
                    eprint!("\t update:\t<none>\n");
                },
                Err(Error::ArgsNoEntry)           => eprintln!("error: entry unspecified or not given as a nonnegative integer"),
                Err(Error::ArgsNoFeed)            => eprintln!("error: feed unspecified"),
                Err(Error::BadContent)            => eprintln!("error: malformed XML near \"content\" or \"description\" element"),
                Err(Error::BadEnclosure)          => eprintln!("error: malformed XML near \"enclosure\" element"),
                Err(Error::EnvNoHome)             => eprintln!("error: $HOME environment variable not set"),
                Err(Error::Io(e))                 => eprintln!("error (io): {:?}", e),
                Err(Error::KeyNoMatch)            => eprintln!("error: given feed_key did not match any feeds"),
                Err(Error::ParserAttributeAbsent) => eprintln!("error: XML attribute missing"),
                Err(Error::ParserDateAbsent)      => eprintln!("error: XML date element missing"),
                Err(Error::Xml(e))                => eprintln!("error (xml-rs): {:?}", e),
                Err(Error::XmlUnexpectedEOF)      => eprintln!("error: unexpected end-of-file in XML"),
            }
        },
        Err(Error::Io(e))      => eprintln!("error (io): {:?}", e),
        Err(Error::KeyNoMatch) => eprintln!("error: given feed_key did not match any feeds"),
        Err(Error::Xml(e))     => eprintln!("error (xml-rs): {:?}", e),
        _ /* default */        => eprintln!("error: an error was encountered that the developer did not anticipate, but should've"),
    };
    process::exit(1);
}
