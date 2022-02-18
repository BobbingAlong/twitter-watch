use chrono::{Date, DateTime, TimeZone, Utc};
use clap::Parser;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::path::Path;

const REPORTED_LIMIT: usize = 10;
const SCREEN_NAMES_FOLLOWERS_COUNT_LIMIT: usize = 200;
const SUSPENSIONS_FOLLOWERS_COUNT_LIMIT: usize = 200;
const HEADER_DATE_FORMAT: &str = "%e %B %Y";

fn main() -> Result<(), Error> {
    let opts: Opts = Opts::parse();

    match opts.command {
        Command::ScreenNames { base } => {
            let base_path = Path::new(&base);
            let mut data = csv::Reader::from_reader(File::open(base_path.join("data.csv"))?);

            let mut by_date: HashMap<Date<Utc>, Vec<ScreenNameRecord>> = HashMap::new();

            for result in data.records() {
                let record = ScreenNameRecord::try_from(result?)?;
                let date = record.timestamp.date();

                let records = by_date.entry(date).or_default();
                records.push(record);
            }

            let mut date_records = by_date
                .into_iter()
                .map(|(date, mut records)| {
                    records.sort_by_key(|record| (Reverse(record.followers_count), record.user_id));
                    (date, records)
                })
                .collect::<Vec<_>>();

            date_records.sort_by_key(|(date, _)| Reverse(*date));

            println!("# Screen name changes");
            println!("This report tracks screen name changes for several million far-right and far-right adjacent accounts on Twitter");
            println!("(including a lot of crypto / NFT shit, some spam, antivaxxers, etc.).\n");
            println!("This page presents the last ten days of available data for all users with more than {} followers.", SCREEN_NAMES_FOLLOWERS_COUNT_LIMIT);
            println!("Please note:");
            println!("* The date listed indicates the day the change was detected, and in some cases it may have happened earlier.");
            println!("* The \"Twitter ID\" column provides a stable link for the account in cases where the screen name has been changed again.");
            println!("* Some accounts may have been suspended or deactivated since being added to the report.");
            println!("* There's a lot of potentially offensive content here, including racial slurs and obscenity.\n");
            println!("The full history of all detected changes for all tracked users is available in the [`data.csv`](./data.csv) file.");

            println!("## Contents");

            for (date, records) in date_records.iter().take(REPORTED_LIMIT) {
                println!(
                    "* [{} ({} changes found)](#{})",
                    date.format(HEADER_DATE_FORMAT),
                    records.len(),
                    date.format(HEADER_DATE_FORMAT)
                        .to_string()
                        .trim()
                        .replace(" ", "-")
                );
            }

            for (date, records) in date_records.into_iter().take(REPORTED_LIMIT) {
                println!("\n## {}", date.format(HEADER_DATE_FORMAT));
                println!(
                    "Found {} screen name changes, with {} included here.",
                    records.len(),
                    records
                        .iter()
                        .filter(
                            |record| record.followers_count >= SCREEN_NAMES_FOLLOWERS_COUNT_LIMIT
                        )
                        .count()
                );
                println!("<table>");
                println!("<tr><th></th><th align=\"left\">Twitter ID</th><th align=\"left\">Previous screen name</th>");
                println!("<th align=\"left\">New screen name</th><th align=\"left\">Status</th><th align=\"left\">Follower count</th></tr>");
                for record in records.into_iter().take_while(|record| {
                    record.followers_count >= SCREEN_NAMES_FOLLOWERS_COUNT_LIMIT
                }) {
                    let image_url =
                        make_profile_image_thumbnail_url(&record.profile_image_url, &base_path);
                    let img = format!(
                        "<a href=\"{}\"><img src=\"{}\" width=\"40px\" height=\"40px\" align=\"center\"/></a>",
                        record.profile_image_url, image_url
                    );
                    let id_link = format!(
                        "<a href=\"https://twitter.com/intent/user?user_id={}\">{}</a>",
                        record.user_id, record.user_id
                    );
                    let screen_name_link = format!(
                        "<a href=\"https://twitter.com/{}\">{}</a>",
                        record.new_screen_name, record.new_screen_name
                    );
                    let mut status = String::new();
                    if record.protected {
                        status.push_str("🔒");
                    }
                    if record.verified {
                        status.push_str("✔️");
                    }

                    println!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td align=\"center\">{}</td><td>{}</td></tr>",
                        img,
                        id_link,
                        record.previous_screen_name,
                        screen_name_link,
                        status,
                        record.followers_count
                    );
                }
                println!("</table>");
            }
        }
        Command::Suspensions { base } => {
            let base_path = Path::new(&base);
            let mut data = csv::Reader::from_reader(File::open(base_path.join("data.csv"))?);

            let mut by_date: HashMap<Date<Utc>, Vec<Option<SuspensionRecord>>> = HashMap::new();

            for result in data.records() {
                let csv_record = result?;
                let (record, date) = if csv_record[3].is_empty() {
                    (
                        None,
                        Utc.timestamp(csv_record[0].parse::<i64>().unwrap(), 0)
                            .date(),
                    )
                } else {
                    let record = SuspensionRecord::try_from(csv_record)?;
                    let date = record.timestamp.date().clone();
                    (Some(record), date)
                };

                let records = by_date.entry(date).or_default();
                records.push(record);
            }

            let mut date_records = by_date
                .into_iter()
                .map(|(date, records)| {
                    let unknown_count = records
                        .iter()
                        .filter(|maybe_record| maybe_record.is_none())
                        .count();

                    let mut new_records = records
                        .into_iter()
                        .filter_map(|maybe_record| maybe_record)
                        .collect::<Vec<_>>();

                    new_records
                        .sort_by_key(|record| (Reverse(record.followers_count), record.user_id));

                    (date, new_records, unknown_count)
                })
                .collect::<Vec<_>>();

            date_records.sort_by_key(|(date, _, _)| Reverse(*date));

            println!("# Suspensions");
            println!("This report tracks suspensions for several million far-right and far-right adjacent accounts on Twitter");
            println!("(including a lot of crypto / NFT shit, some spam, antivaxxers, etc.).\n");
            println!("This page presents the last ten days of available data for all users with more than {} followers.", SUSPENSIONS_FOLLOWERS_COUNT_LIMIT);
            println!("Please note:");
            println!("* The dates listed indicate when the suspension or reversal was detected, and in some cases it may have happened earlier.");
            println!("* In some cases the screen name may have been changed before the account was suspended.");
            println!("* There's a lot of potentially offensive content here, including racial slurs and obscenity.\n");
            println!("The full history of all detected suspensions for all tracked users is available in the [`data.csv`](./data.csv) file.");

            println!("## Contents");

            for (date, records, unknown_count) in date_records.iter().take(REPORTED_LIMIT) {
                println!(
                    "* [{} ({} suspensions found)](#{})",
                    date.format(HEADER_DATE_FORMAT),
                    records.len() + unknown_count,
                    date.format(HEADER_DATE_FORMAT)
                        .to_string()
                        .trim()
                        .replace(" ", "-")
                );
            }

            for (date, records, unknown_count) in date_records.into_iter().take(REPORTED_LIMIT) {
                println!("\n## {}", date.format(HEADER_DATE_FORMAT));
                println!(
                    "Found {} suspensions, with {} included here.",
                    records.len() + unknown_count,
                    records
                        .iter()
                        .filter(|record| record.followers_count >= SUSPENSIONS_FOLLOWERS_COUNT_LIMIT)
                        .count()
                );
                println!("<table>");
                println!("<tr><th></th><th align=\"left\">Twitter ID</th><th align=\"left\">Screen name</th>");
                println!("<th align=\"left\">Created</th><th align=\"left\">Reversed</th>");
                println!(
                    "<th align=\"left\">Status</th><th align=\"left\">Follower count</th></tr>"
                );
                for record in records.into_iter().take_while(|record| {
                    record.followers_count >= SUSPENSIONS_FOLLOWERS_COUNT_LIMIT
                }) {
                    let image_url =
                        make_profile_image_thumbnail_url(&record.profile_image_url, &base_path);
                    let img = format!(
                        "<a href=\"{}\"><img src=\"{}\" width=\"40px\" height=\"40px\" align=\"center\"/></a>",
                        record.profile_image_url, image_url
                    );
                    let id_link = format!(
                        "<a href=\"https://twitter.com/intent/user?user_id={}\">{}</a>",
                        record.user_id, record.user_id
                    );
                    let screen_name_link = format!(
                        "<a href=\"https://twitter.com/{}\">{}</a>",
                        record.screen_name, record.screen_name
                    );

                    let created_at = record.created_at.format("%Y-%m-%d");
                    let reversal = record
                        .reversal
                        .map(|value| format!("{}", value.format("%Y-%m-%d")))
                        .unwrap_or_default();

                    let mut status = String::new();
                    if record.protected {
                        status.push_str("🔒");
                    }
                    if record.verified {
                        status.push_str("✔️");
                    }

                    println!(
                        "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td align=\"center\">{}</td><td>{}</td></tr>",
                        img,
                        id_link,
                        screen_name_link,
                        created_at,
                        reversal,
                        status,
                        record.followers_count
                    );
                }
                println!("</table>");
            }
        }
    }

    Ok(())
}

fn make_profile_image_thumbnail_url(profile_image_url: &str, base: &Path) -> String {
    let re =
        regex::Regex::new(r"^https?://([^/]+)/profile_images/(\d+)/(.*)_normal(\.[a-zA-Z0-9-]+)?$")
            .unwrap();

    re.captures(profile_image_url)
        .and_then(|captures| {
            let ((id, name), extension) = captures
                .get(2)
                .map(|m| m.as_str())
                .zip(captures.get(3).map(|m| m.as_str()))
                .zip(captures.get(4).map(|m| m.as_str()))?;

            let path = format!("./thumbnails/{}-{}_400x400{}", id, name, extension);

            if base.join(&path).exists() {
                Some(path)
            } else {
                None
            }
        })
        .unwrap_or(profile_image_url.to_string())
}

struct ScreenNameRecord {
    timestamp: DateTime<Utc>,
    user_id: u64,
    verified: bool,
    protected: bool,
    followers_count: usize,
    previous_screen_name: String,
    new_screen_name: String,
    profile_image_url: String,
}

impl TryFrom<csv::StringRecord> for ScreenNameRecord {
    type Error = Error;

    fn try_from(value: csv::StringRecord) -> Result<Self, Self::Error> {
        if value.len() == 8 {
            let ((((timestamp, user_id), verified), protected), followers_count) = value[0]
                .parse::<i64>()
                .map(|timestamp_s| Utc.timestamp(timestamp_s, 0))
                .ok()
                .zip(value[1].parse::<u64>().ok())
                .zip(value[2].parse::<bool>().ok())
                .zip(value[3].parse::<bool>().ok())
                .zip(value[4].parse::<usize>().ok())
                .ok_or_else(|| Error::InvalidScreenNamesRecord(value.clone()))?;

            Ok(Self {
                timestamp,
                user_id,
                verified,
                protected,
                followers_count,
                previous_screen_name: value[5].to_string(),
                new_screen_name: value[6].to_string(),
                profile_image_url: value[7].to_string(),
            })
        } else {
            Err(Error::InvalidScreenNamesRecord(value))
        }
    }
}

struct SuspensionRecord {
    timestamp: DateTime<Utc>,
    reversal: Option<DateTime<Utc>>,
    user_id: u64,
    created_at: DateTime<Utc>,
    screen_name: String,
    verified: bool,
    protected: bool,
    followers_count: usize,
    profile_image_url: String,
}

impl TryFrom<csv::StringRecord> for SuspensionRecord {
    type Error = Error;

    fn try_from(value: csv::StringRecord) -> Result<Self, Self::Error> {
        if value.len() == 9 {
            let (
                (((((timestamp, reversal), user_id), created_at), verified), protected),
                followers_count,
            ) = value[0]
                .parse::<i64>()
                .map(|timestamp_s| Utc.timestamp(timestamp_s, 0))
                .ok()
                .zip(if value[1].is_empty() {
                    Some(None)
                } else {
                    value[1]
                        .parse::<i64>()
                        .map(|timestamp_s| Some(Utc.timestamp(timestamp_s, 0)))
                        .ok()
                })
                .zip(value[2].parse::<u64>().ok())
                .zip(
                    value[3]
                        .parse::<i64>()
                        .map(|timestamp_s| Utc.timestamp(timestamp_s, 0))
                        .ok(),
                )
                .zip(value[5].parse::<bool>().ok())
                .zip(value[6].parse::<bool>().ok())
                .zip(value[7].parse::<usize>().ok())
                .ok_or_else(|| Error::InvalidScreenNamesRecord(value.clone()))?;

            Ok(Self {
                timestamp,
                reversal,
                user_id,
                created_at,
                screen_name: value[4].to_string(),
                verified,
                protected,
                followers_count,
                profile_image_url: value[8].to_string(),
            })
        } else {
            Err(Error::InvalidSuspensionsRecord(value))
        }
    }
}

#[derive(Debug, Parser)]
#[clap(name = "report", version, author)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    ScreenNames {
        /// Screen name directory
        #[clap(long, default_value = "screen-names/")]
        base: String,
    },
    Suspensions {
        /// Suspensions directory
        #[clap(long, default_value = "suspensions/")]
        base: String,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("CSV error")]
    Csv(#[from] csv::Error),
    #[error("Invalid screen names record")]
    InvalidScreenNamesRecord(csv::StringRecord),
    #[error("Invalid suspensions record")]
    InvalidSuspensionsRecord(csv::StringRecord),
}
