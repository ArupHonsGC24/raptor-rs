
// Copied from gtfs_structures::serde_helpers, which are private :(
// TODO: Make const for fun?
pub fn parse_time_impl(h: &str, m: &str, s: &str) -> Result<u32, std::num::ParseIntError> {
    let hours: u32 = h.parse()?;
    let minutes: u32 = m.parse()?;
    let seconds: u32 = s.parse()?;
    Ok(hours * 3600 + minutes * 60 + seconds)
}

pub fn parse_time(s: &str) -> Result<u32, gtfs_structures::Error> {
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

pub fn get_time_str(time: u32) -> String {
    let hours = time / 3600;
    let minutes = (time % 3600) / 60;
    let seconds = time % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}