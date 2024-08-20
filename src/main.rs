use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::create_dir_all;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use rss::Channel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SeenItems {
    feeds: HashMap<String, HashSet<String>>,
}

impl SeenItems {
    fn new() -> Self {
        SeenItems {
            feeds: HashMap::new(),
        }
    }

    fn load_from_file(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let seen_items: SeenItems = serde_json::from_reader(reader)?;
        Ok(seen_items)
    }

    fn save_to_file(&self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::create(file_path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }
}

fn load_feed_urls(file_path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let urls: Vec<String> = reader.lines().map_while(Result::ok).collect();
    Ok(urls)
}

fn download_file(url: &str, directory: &str) -> Result<(), Box<dyn Error>> {
    let response = reqwest::blocking::get(url)?;
    let file_name = url.split('/').last().unwrap_or("downloaded_file");
    let file_path = Path::new(directory).join(file_name);

    let mut file = File::create(file_path)?;
    let content = response.bytes()?;
    file.write_all(&content)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let feed_urls_file = "feed_urls.txt"; // File containing feed URLs
    let seen_items_file = "seen_items.json"; // File for storing seen items

    // Load feed URLs from the text file
    let feed_urls = load_feed_urls(feed_urls_file)?;

    // Load seen items from file
    let mut seen_items = if fs::metadata(seen_items_file).is_ok() {
        SeenItems::load_from_file(seen_items_file)?
    } else {
        SeenItems::new()
    };

    // Iterate over each feed URL
    for url in feed_urls {
        // Fetch the RSS feed
        let response = reqwest::blocking::get(&url)?.text()?;
        let channel = Channel::read_from(response.as_bytes())?;

        // Print the title of the feed
        println!("Feed Title: {}", channel.title());

        // Create a directory for the feed
        let feed_directory = &channel.title();
        create_dir_all(feed_directory)?;

        // Initialize the seen items for this feed if it doesn't exist
        let feed_seen_items = seen_items
            .feeds
            .entry(url.clone())
            .or_insert_with(HashSet::new);

        // Check for new items
        for item in channel.items() {
            let link = item.link().unwrap_or("No link").to_string();
            if !feed_seen_items.contains(&link) {
                println!("New Item: {}", item.title().unwrap_or("No title"));
                println!("Link: {link}");
                println!(
                    "Description: {}",
                    item.description().unwrap_or("No description")
                );
                println!("---");

                // Add the new item to the seen items
                feed_seen_items.insert(link.clone());

                // Check for external file links in the item
                if let Some(enclosure) = item.enclosure() {
                    let file_url = enclosure.url();
                    println!("Downloading file: {file_url}");
                    if let Err(e) = download_file(file_url, feed_directory) {
                        eprintln!("Failed to download file: {e}");
                    }
                }
            }
        }
    }

    // Save the updated seen items back to the file
    seen_items.save_to_file(seen_items_file)?;

    Ok(())
}
