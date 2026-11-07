use std::{str, time::Duration};

use anyhow::{Ok, Result, bail, ensure};
pub use serialport::available_ports;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

struct SmallString {
    data: [u8; 16],
    length: u8,
}

impl SmallString {
    pub fn new(data: [u8; 16]) -> Self {
        let trim = data.trim_ascii();

        let mut data = [0; 16];
        data[..trim.len()].copy_from_slice(&trim);

        Self {
            data,
            length: trim.len() as u8,
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.data[..self.length as usize]) }
    }
}

pub struct Scale {
    device: RawScale,

    model: SmallString,
    cell_serial: SmallString,
}

pub enum WeighMode {
    VeryStable,
    Stable,
    Unstable,
    VeryUnstable,
}

impl WeighMode {
    fn command(&self) -> u8 {
        match self {
            WeighMode::VeryStable => b'K',
            WeighMode::Stable => b'L',
            WeighMode::Unstable => b'M',
            WeighMode::VeryUnstable => b'N',
        }
    }
}

struct RawScale {
    inner: Box<dyn SerialPort>,
}

impl RawScale {
    pub fn connect_default(path: &str) -> Result<Self> {
        let inner = serialport::new(path, 1200)
            .data_bits(DataBits::Seven)
            .flow_control(FlowControl::Software)
            .parity(Parity::Odd)
            .stop_bits(StopBits::One)
            .timeout(Duration::from_secs(60))
            .open()?;

        Ok(Self { inner })
    }

    pub fn command_send_only(&mut self, command: &[u8]) -> Result<()> {
        self.inner.write(&[0x1b])?;
        self.inner.write(command)?;
        self.inner.write(b"\r\n")?;
        self.inner.flush()?;

        Ok(())
    }

    pub fn command<const N: usize>(&mut self, command: &[u8]) -> Result<[u8; N]> {
        self.command_send_only(command)?;

        let mut out = [0; N];
        self.inner.read_exact(&mut out)?;

        Ok(out)
    }
}

impl Scale {
    pub fn connect_default(path: &str) -> Result<Self> {
        let mut device = RawScale::connect_default(path)?;

        let model = SmallString::new(device.command(b"x1_")?);
        let cell_serial = SmallString::new(device.command(b"x2_")?);

        Ok(Self {
            device,
            model,
            cell_serial,
        })
    }

    pub fn model(&self) -> &str {
        self.model.as_str()
    }

    pub fn cell_serial(&self) -> &str {
        self.cell_serial.as_str()
    }

    pub fn set_weigh_mode(&mut self, mode: WeighMode) -> Result<()> {
        self.device.command_send_only(&[mode.command()])?;
        Ok(())
    }

    pub fn tare(&mut self) -> Result<()> {
        self.device.command_send_only(b"T")?;
        Ok(())
    }

    pub fn block_keys(&mut self) -> Result<()> {
        self.device.command_send_only(b"O")?;
        Ok(())
    }

    pub fn release_keys(&mut self) -> Result<()> {
        self.device.command_send_only(b"R")?;
        Ok(())
    }

    pub fn weigh(&mut self) -> Result<WeighResult> {
        let resp = self.device.command::<22>(b"P")?;
        // println!("DBG {:?}", String::from_utf8_lossy(&resp));

        ensure!(&resp[..6] == b"N     ", "Scale in unexpected mode");

        let sign = match resp[6] {
            b' ' => 0,
            b'+' => 1,
            b'-' => -1,
            _ => bail!("Scale in unexpected mode"),
        };

        let value = unsafe { str::from_utf8_unchecked(resp[7..17].trim_ascii()) }.parse::<f32>()?
            * sign as f32;
        let unit = Unit::from_code(unsafe { str::from_utf8_unchecked(resp[17..].trim_ascii()) });

        if let Some(unit) = unit {
            Ok(WeighResult::Weight { value, unit })
        } else {
            Ok(WeighResult::Preview { value })
        }
    }
}

#[derive(Debug)]
pub enum WeighResult {
    Preview { value: f32 },
    Weight { value: f32, unit: Unit },
}

#[derive(Debug)]
pub enum Unit {
    Gram,
}

impl Unit {
    fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "g" => Self::Gram,
            _ => return None,
        })
    }
}
