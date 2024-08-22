use chrono::NaiveDate;
use gtfs_structures::{Gtfs, Trip};

use crate::network::Timestamp;

pub trait OptionExt<T> {
    fn is_none_or<F: FnOnce(T) -> bool>(self, f: F) -> bool;
}

impl<T> OptionExt<T> for Option<T> {
    fn is_none_or<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        self.map(f).unwrap_or(true)
    }
}

pub const fn const_unwrap<T: Copy>(x: Option<T>) -> T {
    if let Some(x) = x { x } else { panic!("Failed to const unwrap.") }
}

// A fast way to check a buffer is all zeros (https://stackoverflow.com/questions/65367552/how-to-efficiently-check-a-vecu8-to-see-if-its-all-zeros).
pub fn is_zero(buf: &[bool]) -> bool {
    let (prefix, aligned, suffix) = unsafe { buf.align_to::<u128>() };

    prefix.iter().all(|&x| x == false)
        && aligned.iter().all(|&x| x == 0)
        && suffix.iter().all(|&x| x == false)
}

pub const fn get_size_bits<T>() -> usize {
    std::mem::size_of::<T>() * 8
}

pub fn get_short_stop_name(stop: &str) -> &str {
    // Convert "Laburnum Railway Station (Blackburn)" to "Laburnum", and "Noble Park Railway Station (Noble Park)" to "Noble Park", etc.
    stop.split(" Railway Station").next().unwrap()
}

pub fn does_trip_run(gtfs: &Gtfs, trip: &Trip, date: NaiveDate) -> bool {
    if let Some(calender) = gtfs.calendar.get(trip.service_id.as_str()) {
        calender.valid_weekday(date) && calender.start_date <= date && date <= calender.end_date
    } else if let Some(calender_dates) = gtfs.calendar_dates.get(trip.service_id.as_str()) {
        calender_dates.iter().any(|calender_date| calender_date.date == date)
    } else {
        assert!(false, "Trip {} does not have a valid service_id", trip.id);
        false
    }
}

// Copied from gtfs_structures::serde_helpers, which are private :(
fn parse_time_impl(h: &str, m: &str, s: &str) -> Result<Timestamp, std::num::ParseIntError> {
    let hours: u32 = h.parse()?;
    let minutes: u32 = m.parse()?;
    let seconds: u32 = s.parse()?;
    Ok(hours * 3600 + minutes * 60 + seconds)
}

pub fn parse_time(s: &str) -> Result<Timestamp, gtfs_structures::Error> {
    if s.len() < 7 {
        Err(gtfs_structures::Error::InvalidTime(s.to_owned()))
    } else {
        let parts: Vec<&str> = s.split(':').collect();

        if parts.len() != 3 {
            return Err(gtfs_structures::Error::InvalidTime(s.to_owned()));
        }

        let sec = parts[2];
        let min = parts[1];
        let hour = parts[0];

        if min.len() != 2 || sec.len() != 2 {
            return Err(gtfs_structures::Error::InvalidTime(s.to_owned()));
        }

        parse_time_impl(hour, min, sec).map_err(|_| gtfs_structures::Error::InvalidTime(s.to_owned()))
    }
}

pub fn get_time_str(time: Timestamp) -> String {
    let hours = time / 3600;
    let minutes = (time % 3600) / 60;
    let seconds = time % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
