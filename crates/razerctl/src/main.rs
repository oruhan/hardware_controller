use std::env;
use std::io::{self, Write};

use razer::{DaV3HS, PollingRate};

fn usage() {
    eprintln!("Usage:\n  razerctl status\n  razerctl set-polling-rate <125|500|1000>");
}

fn parse_rate(value: &str) -> Result<PollingRate, String> {
    match value {
        "125" => Ok(PollingRate::Hz125),
        "500" => Ok(PollingRate::Hz500),
        "1000" => Ok(PollingRate::Hz1000),
        _ => Err(format!("unsupported polling rate: {value}")),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.is_empty() || matches!(arguments[0].as_str(), "-h" | "--help") {
        usage();
        return Ok(());
    }

    let mut mouse = DaV3HS::open()?;

    match arguments.as_slice() {
        [command] if command == "status" => {
            let battery = mouse.battery()?;
            let polling_rate = mouse.get_polling_rate()?;
            println!(
                "connection: {}",
                if mouse.is_wireless() {
                    "2.4 GHz"
                } else {
                    "USB"
                }
            );
            println!("battery: {}% (raw: {})", battery.percentage, battery.raw);
            println!(
                "charging: {}",
                if battery.is_charging() { "yes" } else { "no" }
            );
            println!("polling rate: {} Hz", polling_rate.hz());
        }
        [command, value] if command == "set-polling-rate" => {
            let rate = parse_rate(value).map_err(io::Error::other)?;
            mouse.set_polling_rate(rate)?;
            println!("polling rate set to {} Hz", rate.hz());
        }
        _ => {
            usage();
            return Err(io::Error::other("invalid command").into());
        }
    }

    io::stdout().flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_rates() {
        assert_eq!(parse_rate("125"), Ok(PollingRate::Hz125));
        assert_eq!(parse_rate("500"), Ok(PollingRate::Hz500));
        assert_eq!(parse_rate("1000"), Ok(PollingRate::Hz1000));
    }

    #[test]
    fn rejects_unsupported_rates() {
        assert!(parse_rate("2000").is_err());
    }
}
