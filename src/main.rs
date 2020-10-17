fn main() -> Result<(), std::io::Error> {
    let config = rsst::Config::new(
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
