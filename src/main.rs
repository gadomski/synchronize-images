extern crate chrono;
#[macro_use]
extern crate failure;

use chrono::{DateTime, Utc};
use std::path::Path;
use std::str::FromStr;

fn main() {
    println!("Hello, world!");
}

/// The errors that can be produced by this executable.
#[derive(Debug, Fail)]
#[fail(display = "generic error")]
struct Error {}

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
    timestamp: DateTime<Utc>,
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
            .map(|result| {
                result
                    .map_err(failure::Error::from)
                    .and_then(|line| line.parse().map_err(failure::Error::from))
            })
            .collect::<Result<Vec<EventMarker>, _>>()?;
        unimplemented!()
    }
}

impl FromStr for EventMarker {
    type Err = Error;
    fn from_str(s: &str) -> Result<EventMarker, Error> {
        unimplemented!()
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
