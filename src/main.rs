use std::env::args;
use std::str::FromStr;

use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use colored::Colorize;
use harvest_api::HarvestClient;

fn get_expected_weekdays(start_date: &NaiveDate, end_date: &NaiveDate) -> f64 {
    const HOURS_PER_DAY: f64 = 7.5;

    let mut current_date = start_date.clone();

    let mut total_days: usize = 0;
    while current_date <= *end_date {
        total_days += match current_date.weekday() {
            Weekday::Sat | Weekday::Sun => 0,
            _ => 1
        };
        current_date = current_date + Duration::days(1);
    }

    total_days as f64 * HOURS_PER_DAY
}


#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = args().collect();

    let start_date_arg = args
        .get(1)
        .expect("No start date found.");

    let start_date =
        NaiveDate::from_str(start_date_arg)
            .unwrap()
            .format("%Y-%m-%d");
    let start_date_str = start_date.as_str();


    // Set today as date if none is provided + adjust provided date to today if it's in the future.
    let today = Utc::now().date_naive();
    let end_date_string;
    match args.get(2) {
        None => {
            end_date_string = format!("{}", today.format("%Y-%m-%d"))
        }
        Some(s) => {
            let parsed_date = NaiveDate::from_str(s).unwrap();
            let date = if parsed_date <= today {
                parsed_date
            } else {
                println!("Changing end date to today, as provided date is in the future.");
                today
            };

            end_date_string = format!("{}", date.format("%Y-%m-%d"))
        }
    };
    let end_date_str = end_date_string.as_str();

    let harvest_client = HarvestClient::from_env();
    let time_entries = harvest_client
        .list_time_entries()
        .from(start_date_str)
        .to(end_date_str)
        .send();

    let expected_hours = get_expected_weekdays(start_date, end_date_str);
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