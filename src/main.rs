use std::env::args;
use std::str::FromStr;

use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use colored::Colorize;
use harvest_api::HarvestClient;

fn get_expected_weekdays(start_date: &NaiveDate, end_date: &NaiveDate) -> f64 {
    let hours_per_day = match std::env::var("HARVEST_HOURS_PER_DAY") {
        Ok(s) => s
            .parse::<f64>()
            .expect("Hours per day is not in a numeric format."),
        Err(..) => {
            println!("Hours per day not found. Using 7.5");
            7.5
        }
    };

    let mut current_date = start_date.clone();

    let mut total_days: usize = 0;
    while current_date < *end_date {
        total_days += match current_date.weekday() {
            Weekday::Sat | Weekday::Sun => 0,
            _ => 1
        };
        current_date = current_date + Duration::days(1);
    }

    total_days as f64 * hours_per_day
}


#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = args().collect();

    let today = Utc::now().date_naive();
    let first_day_of_the_year = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();

    // Set today as end date if none is provided + adjust provided date to today if it's in the future.
    let end_date = match args.get(2) {
        None => today,
        Some(s) => {
            let parsed_date = NaiveDate::from_str(s).unwrap();
            if parsed_date <= today {
                parsed_date
            } else {
                println!("Changing end date to today, as provided date is in the future.");
                today
            }
        }
    };

    // Set January 1st of current year as start date if no other is provided.
    let start_date = match args.get(1) {
        None => first_day_of_the_year,
        Some(s) => {
            let parsed_date = NaiveDate::from_str(s).unwrap();
            if parsed_date <= today && parsed_date <= end_date {
                parsed_date
            } else {
                println!("{}", format!("Provided start date is after the end date. Setting start date to January 1st. {}", today.year()).red());
                first_day_of_the_year
            }
        }
    };

    let start_date_str = format!("{}", start_date.format("%Y-%m-%d"));
    let end_date_str = format!("{}", end_date.format("%Y-%m-%d"));

    let harvest_client = HarvestClient::from_env();
    let time_entries = harvest_client
        .list_time_entries()
        .from(start_date_str.as_str())
        .to(end_date_str.as_str())
        .send();

    let expected_hours = get_expected_weekdays(&start_date, &end_date);
    let actual_hours = time_entries
        .await
        .expect("Invalid date. Month and day needs leading zeros")
        .time_entries
        .iter()
        .map(|te| te.hours.unwrap())
        .sum::<f64>();

    let flex_balance = actual_hours - expected_hours;

    println!("Start date: {} - End date: {}", start_date_str, end_date_str);
    println!("Expected hours: {expected_hours}");
    println!("Actual hours: {actual_hours}");

    if flex_balance >= 0.0 {
        println!("{}", format!("{flex_balance} hour(s) above expected").green());
    } else {
        println!("{}", format!("{flex_balance} hour(s) below expected").red())
    };

    Ok(())
}