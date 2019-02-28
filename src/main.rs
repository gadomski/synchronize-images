extern crate chrono;
extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
extern crate regex;

use chrono::{DateTime, Utc};
use std::path::Path;
use std::str::FromStr;

fn main() -> Result<(), failure::Error> {
    use clap::{App, Arg};
    let matches = App::new("Synchronize images")
        .arg(
            Arg::with_name("SYNCHRO")
                .help("The SYNCHRO file from Apps")
                .required(true)
                .index(1),
        )
        .get_matches();
    let _synchro = Synchro::from_path(matches.value_of("SYNCHRO").unwrap())?;
    Ok(())
}

/// The errors that can be produced by this executable.
#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "this string could not be parsed as an event marker: {}", _0)]
    InvalidEventMarker(String),
}

/// A "SYNCRO" file from Apps.
///
/// These files contain the timestamps of each image record.
#[derive(Debug)]
struct Synchro {
    event_markers: Vec<EventMarker>,
}

/// An event marker.
///
/// Links a timestamp and a event marker number.
#[derive(Debug)]
struct EventMarker {
    datetime: DateTime<Utc>,
    number: i32,
}

impl Synchro {
    /// Reads in a synchro file from a path.
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Synchro, failure::Error> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let reader = BufReader::new(File::open(path)?);
        let event_markers = reader
            .lines()
            .filter_map(|result| match result {
                Ok(line) => {
                    if line.is_empty() || line.starts_with('#') {
                        None
                    } else {
                        Some(line.parse().map_err(failure::Error::from))
                    }
                }
                Err(err) => Some(Err(err.into())),
            })
            .collect::<Result<Vec<EventMarker>, _>>()?;
        Ok(Synchro {
            event_markers: event_markers,
        })
    }
}

impl FromStr for EventMarker {
    type Err = failure::Error;
    fn from_str(s: &str) -> Result<EventMarker, failure::Error> {
        use chrono::{NaiveDate, NaiveTime, TimeZone};
        use regex::Regex;

        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(?P<date>\d{4}/\d{2}/\d{2})\s+(?P<time>\d{2}:\d{2}:\d{2}.\d{4})\s+(?P<number>\d+)$").unwrap();
        }
        let captures = RE
            .captures(s)
            .ok_or(Error::InvalidEventMarker(s.to_string()))?;
        let date = NaiveDate::parse_from_str(captures.name("date").unwrap().as_str(), "%Y/%m/%d")?;
        let time =
            NaiveTime::parse_from_str(captures.name("time").unwrap().as_str(), "%H:%M:%S.%f")?;
        let number = captures.name("number").unwrap().as_str().parse()?;
        Ok(EventMarker {
            datetime: Utc.from_utc_datetime(&date.and_time(time)),
            number: number,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_synchro() {
        Synchro::from_path("tests/data/synchro.xpf").unwrap();
    }
}
