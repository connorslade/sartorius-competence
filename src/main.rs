use anyhow::Result;
use sartorius_competence::{Scale, WeighMode};

fn main() -> Result<()> {
    let mut scale = Scale::connect_default("/dev/ttyUSB0")?;

    println!("Connected to {} ({})", scale.model(), scale.cell_serial());

    scale.set_weigh_mode(WeighMode::Stable)?;
    loop {
        println!("⇒ {:?}", scale.weigh()?);
    }
}
