extern crate chrono;
extern crate clap;
extern crate csv;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use chrono::{DateTime, Utc};
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::vec::IntoIter;

fn main() -> Result<(), failure::Error> {
    use clap::{App, Arg};
    let matches = App::new("Synchronize images")
        .arg(
            Arg::with_name("SYNCHRO")
                .help("The SYNCHRO file from Apps")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("IMAGES")
                .help("A file with one image name per line")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("TRAJECTORY")
                .help("A trajectory file, probably created by reading SBET to text with PDAL")
                .required(true)
                .index(3),
        )
        .get_matches();
    let synchro = Synchro::from_path(matches.value_of("SYNCHRO").unwrap())?;
    let images = read_image_names(matches.value_of("IMAGES").unwrap())?;
    let trajectory = Trajectory::from_path(matches.value_of("TRAJECTORY").unwrap())?;

    let _synchronizer = Synchronizer::new(synchro, images)?;
    Ok(())
}

/// The structure that is used to syncronize the synchro file, the image names, and the trajectory.
#[derive(Debug)]
struct Synchronizer {
    synchro: IntoIter<EventMarker>,
    images: IntoIter<String>,
}

/// The errors that can be produced by this executable.
#[derive(Debug, Fail, PartialEq)]
enum Error {
    CountMismatch {
        synchro: Synchro,
        images: Vec<String>,
    },
    InvalidEventMarker(String),
    GpsWeekTimeSlip {
        before: f64,
        after: f64,
    },
}

/// A "SYNCRO" file from Apps.
///
/// These files contain the timestamps of each image record.
#[derive(Clone, Debug, PartialEq)]
struct Synchro {
    event_markers: Vec<EventMarker>,
}

/// An event marker.
///
/// Links a timestamp and a event marker number.
#[derive(Clone, Debug, PartialEq)]
struct EventMarker {
    datetime: DateTime<Utc>,
    number: i32,
}

/// A trajectory.
#[derive(Debug)]
struct Trajectory {}

/// A position and orientation, with time.
#[derive(Debug, Deserialize)]
struct Position {
    #[serde(alias = "GpsTime")]
    time: Time,

    #[serde(alias = "X")]
    longitude: f64,

    #[serde(alias = "Y")]
    latitude: f64,

    #[serde(alias = "Z")]
    height: f64,

    #[serde(alias = "Roll")]
    roll: f64,

    #[serde(alias = "Pitch")]
    pitch: f64,

    #[serde(alias = "Azimuth")]
    yaw: f64,
}

/// A time enum to capture both GPS week time and real time.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Time {
    /// GPS week time a.k.a. seconds from midnight on Sunday.
    GpsWeekTime(f64),

    /// Real time.
    #[serde(skip_deserializing)]
    Real(DateTime<Utc>),
}

impl Synchronizer {
    /// Creates a new synchronizere.
    fn new(synchro: Synchro, images: Vec<String>) -> Result<Synchronizer, Error> {
        if synchro.len() != images.len() {
            return Err(Error::CountMismatch {
                synchro: synchro,
                images: images,
            });
        }
        Ok(Synchronizer {
            synchro: synchro.into_iter(),
            images: images.into_iter(),
        })
    }
}

impl Iterator for Synchronizer {
    type Item = (EventMarker, String);
    fn next(&mut self) -> Option<(EventMarker, String)> {
        unimplemented!()
    }
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

    fn len(&self) -> usize {
        self.event_markers.len()
    }
}

impl IntoIterator for Synchro {
    type Item = EventMarker;
    type IntoIter = IntoIter<EventMarker>;
    fn into_iter(self) -> IntoIter<EventMarker> {
        self.event_markers.into_iter()
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

/// Read image names from a file.
///
/// One file name per line.
fn read_image_names<P: AsRef<Path>>(path: P) -> Result<Vec<String>, failure::Error> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    BufReader::new(File::open(path)?)
        .lines()
        .map(|result| result.map_err(failure::Error::from))
        .collect::<Result<Vec<String>, _>>()
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::CountMismatch {
                ref synchro,
                ref images,
            } => write!(
                f,
                "count mismatch: synchro={}, images={}",
                synchro.len(),
                images.len()
            ),
            Error::InvalidEventMarker(ref s) => {
                write!(f, "could not parse string into event marker: {}", s)
            }
            Error::GpsWeekTimeSlip { before, after } => {
                write!(f, "gps week time slip: before={}, after={}", before, after)
            }
        }
    }
}

impl Trajectory {
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Trajectory, failure::Error> {
        let positions = read_positions(path)?;
        Trajectory::new(positions).map_err(failure::Error::from)
    }

    fn new(positions: Vec<Position>) -> Result<Trajectory, Error> {
        unimplemented!()
    }
}

fn read_positions<P: AsRef<Path>>(path: P) -> Result<Vec<Position>, failure::Error> {
    use csv::Reader;
    let mut reader = Reader::from_path(path)?;
    reader
        .deserialize()
        .collect::<Result<Vec<Position>, _>>()
        .map_err(failure::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_synchro() {
        Synchro::from_path("tests/data/synchro.xpf").unwrap();
    }

    #[test]
    fn read_image_names() {
        super::read_image_names("tests/data/images.txt").unwrap();
    }

    #[test]
    fn new_synchronizer() {
        let synchro = Synchro::from_path("tests/data/synchro.xpf").unwrap();
        let images = super::read_image_names("tests/data/images.txt").unwrap();
        Synchronizer::new(synchro, images).unwrap();
    }

    #[test]
    fn count_mismatch() {
        let synchro = Synchro::from_path("tests/data/synchro.xpf").unwrap();
        let mut images = super::read_image_names("tests/data/images.txt").unwrap();
        images.pop().unwrap();
        assert_eq!(
            Error::CountMismatch {
                images: images.clone(),
                synchro: synchro.clone(),
            },
            Synchronizer::new(synchro, images).unwrap_err()
        );
    }

    #[test]
    fn read_trajectory() {
        Trajectory::from_path("tests/data/trajectory.txt").unwrap();
    }
}
