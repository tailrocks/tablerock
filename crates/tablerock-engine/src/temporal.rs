pub(crate) const SECONDS_PER_DAY: i64 = 86_400;

pub(crate) fn format_date_from_unix_days(days_since_unix_epoch: i64) -> String {
    let shifted = days_since_unix_epoch + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let year = if (0..=9_999).contains(&year) {
        format!("{year:04}")
    } else if year > 0 {
        format!("+{year}")
    } else {
        format!("-{absolute:04}", absolute = year.unsigned_abs())
    };
    format!("{year}-{month:02}-{day:02}")
}

pub(crate) fn format_unix_timestamp(ticks: i64, scale: u32) -> Option<String> {
    let units = 10_i64.checked_pow(scale)?;
    let seconds = ticks.div_euclid(units);
    let fraction = ticks.rem_euclid(units);
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let time = seconds.rem_euclid(SECONDS_PER_DAY);
    let hours = time / 3_600;
    let minutes = time / 60 % 60;
    let seconds = time % 60;
    let date = format_date_from_unix_days(days);
    if scale == 0 {
        Some(format!("{date}T{hours:02}:{minutes:02}:{seconds:02}Z"))
    } else {
        let width = usize::try_from(scale).ok()?;
        Some(format!(
            "{date}T{hours:02}:{minutes:02}:{seconds:02}.{fraction:0width$}Z"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{format_date_from_unix_days, format_unix_timestamp};

    #[test]
    fn formats_calendar_boundaries_and_scaled_negative_instants() {
        assert_eq!(format_date_from_unix_days(0), "1970-01-01");
        assert_eq!(format_date_from_unix_days(-719_528), "0000-01-01");
        assert_eq!(format_date_from_unix_days(2_933_262), "+10000-12-31");
        assert_eq!(
            format_unix_timestamp(-1, 6).as_deref(),
            Some("1969-12-31T23:59:59.999999Z")
        );
        assert_eq!(
            format_unix_timestamp(1_709_210_096_123_456_789, 9).as_deref(),
            Some("2024-02-29T12:34:56.123456789Z")
        );
        assert!(format_unix_timestamp(0, 19).is_none());
    }
}
