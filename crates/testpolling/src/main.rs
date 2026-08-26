use razer::{ DaV3HS, PollingRate };

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mouse = DaV3HS::open()?;

    //mouse.set_polling_rate(PollingRate::Hz125)?;
    mouse.set_polling_rate(PollingRate::Hz500)?;
    //mouse.set_polling_rate(PollingRate::Hz1000)?;

    Ok(())
}
