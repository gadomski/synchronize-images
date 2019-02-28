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
use std::iter::Skip;
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
    let event_markers = read_synchro(matches.value_of("SYNCHRO").unwrap())?;
    let images = read_image_names(matches.value_of("IMAGES").unwrap())?;
    let mut synchronizer = Synchronizer::new(event_markers, images)?;
    let mut trajectory = Trajectory::from_path(matches.value_of("TRAJECTORY").unwrap())?;

    let (mut event_marker, mut file_name) = synchronizer.next().ok_or(Error::NoEventMarkers)?;
    let (mut before, mut after) = trajectory.next().ok_or(Error::EmptyTrajectory)?;

    let mut records = Vec::new();
    loop {
        let before_datetime = before.datetime(&event_marker);
        let after_datetime = after.datetime(&event_marker);
        if before_datetime <= event_marker.datetime && after_datetime >= event_marker.datetime {
            records.push(Record::new(event_marker, file_name, before, after));
            match synchronizer.next() {
                Some((e, f)) => {
                    event_marker = e;
                    file_name = f;
                }
                None => break,
            };
        } else if before_datetime > event_marker.datetime {
            match synchronizer.next() {
                Some((e, f)) => {
                    event_marker = e;
                    file_name = f;
                }
                None => break,
            };
        } else if after_datetime < event_marker.datetime {
            match trajectory.next() {
                Some((b, a)) => {
                    before = b;
                    after = a;
                }
                None => break,
            };
        } else {
            unreachable!()
        }
    }

    Ok(())
}

/// The output record.
#[derive(Debug, Serialize)]
struct Record {
    file_name: String,
    datetime: DateTime<Utc>,
    longitude: f64,
    latitude: f64,
    height: f64,
    roll: f64,
    pitch: f64,
    yaw: f64,
}

/// A zipped iterator over the event markers and the images.
#[derive(Debug)]
struct Synchronizer {
    event_markers: IntoIter<EventMarker>,
    images: IntoIter<String>,
}

/// The errors that can be produced by this executable.
#[derive(Debug, Fail, PartialEq)]
enum Error {
    CountMismatch {
        event_markers: Vec<EventMarker>,
        images: Vec<String>,
    },
    EmptyTrajectory,
    EventMarkerSlip {
        before: EventMarker,
        after: EventMarker,
    },
    GpsWeekTimeSlip {
        before: Position,
        after: Position,
    },
    InvalidEventMarker(String),
    NoEventMarkers,
}

/// An event marker.
///
/// Links a timestamp and a event marker number.
#[derive(Clone, Copy, Debug, PartialEq)]
struct EventMarker {
    datetime: DateTime<Utc>,
    number: i32,
}

/// A trajectory.
#[derive(Debug)]
struct Trajectory {
    before: IntoIter<Position>,
    after: Skip<IntoIter<Position>>,
}

/// A position and orientation, with time.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
struct Position {
    #[serde(alias = "GpsTime")]
    time: f64,

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

impl Record {
    fn new(
        event_marker: EventMarker,
        file_name: String,
        before: Position,
        after: Position,
    ) -> Record {
        unimplemented!()
    }
}

impl Synchronizer {
    /// Creates a new synchronizere.
    fn new(event_markers: Vec<EventMarker>, images: Vec<String>) -> Result<Synchronizer, Error> {
        if event_markers.len() != images.len() {
            return Err(Error::CountMismatch {
                event_markers: event_markers,
                images: images,
            });
        }
        for (before, after) in event_markers.iter().zip(event_markers.iter().skip(1)) {
            if before.datetime > after.datetime {
                return Err(Error::EventMarkerSlip {
                    before: *before,
                    after: *after,
                });
            }
        }
        Ok(Synchronizer {
            event_markers: event_markers.into_iter(),
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

/// Reads in a synchro file from a path.
fn read_synchro<P: AsRef<Path>>(path: P) -> Result<Vec<EventMarker>, failure::Error> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let reader = BufReader::new(File::open(path)?);
    reader
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
        .collect::<Result<Vec<EventMarker>, _>>()
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
                ref event_markers,
                ref images,
            } => write!(
                f,
                "count mismatch: event_markers={}, images={}",
                event_markers.len(),
                images.len()
            ),
            Error::EmptyTrajectory => write!(f, "empty trajectory"),
            Error::EventMarkerSlip {
                ref before,
                ref after,
            } => write!(
                f,
                "event marker slip: before={}, after={}",
                before.datetime, after.datetime
            ),
            Error::InvalidEventMarker(ref s) => {
                write!(f, "could not parse string into event marker: {}", s)
            }
            Error::GpsWeekTimeSlip { before, after } => write!(
                f,
                "gps week time slip: before={}, after={}",
                before.time, after.time
            ),
            Error::NoEventMarkers => write!(f, "no event markers"),
        }
    }
}

impl Trajectory {
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Trajectory, failure::Error> {
        let positions = read_positions(path)?;
        Trajectory::new(positions).map_err(failure::Error::from)
    }

    fn new(positions: Vec<Position>) -> Result<Trajectory, Error> {
        for (before, after) in positions.iter().zip(positions.iter().skip(1)) {
            if before.time > after.time {
                return Err(Error::GpsWeekTimeSlip {
                    before: *before,
                    after: *after,
                });
            }
        }
        Ok(Trajectory {
            before: positions.clone().into_iter(),
            after: positions.into_iter().skip(1),
        })
    }
}

impl Iterator for Trajectory {
    type Item = (Position, Position);
    fn next(&mut self) -> Option<(Position, Position)> {
        self.before
            .next()
            .and_then(|before| self.after.next().map(|after| (before, after)))
    }
}

impl Position {
    /// Converts this position's gps week time to a real datetime.
    fn datetime(&self, event_marker: &EventMarker) -> DateTime<Utc> {
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
        super::read_synchro("tests/data/synchro.xpf").unwrap();
    }

    #[test]
    fn read_image_names() {
        super::read_image_names("tests/data/images.txt").unwrap();
    }

    #[test]
    fn new_synchronizer() {
        let event_markers = super::read_synchro("tests/data/synchro.xpf").unwrap();
        let images = super::read_image_names("tests/data/images.txt").unwrap();
        Synchronizer::new(event_markers, images).unwrap();
    }

    #[test]
    fn event_marker_time_slip() {
        let mut event_markers = super::read_synchro("tests/data/synchro.xpf").unwrap();
        let event_marker = event_markers.pop().unwrap();
        event_markers.insert(0, event_marker);
        let images = super::read_image_names("tests/data/images.txt").unwrap();
        Synchronizer::new(event_markers, images).unwrap_err();
    }

    #[test]
    fn count_mismatch() {
        let event_markers = super::read_synchro("tests/data/synchro.xpf").unwrap();
        let mut images = super::read_image_names("tests/data/images.txt").unwrap();
        images.pop().unwrap();
        Synchronizer::new(event_markers, images).unwrap_err();
    }

    #[test]
    fn read_trajectory() {
        Trajectory::from_path("tests/data/trajectory.txt").unwrap();
    }

    #[test]
    fn gps_week_time_slip() {
        let mut positions = read_positions("tests/data/trajectory.txt").unwrap();
        let before = positions.pop().unwrap();
        positions.insert(0, before);
        assert_eq!(
            Error::GpsWeekTimeSlip {
                before: positions[0],
                after: positions[1],
            },
            Trajectory::new(positions).unwrap_err()
        );
    }
}
