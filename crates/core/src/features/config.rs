use std::fmt;
use std::str::FromStr;

use anyhow::{Context, Result, ensure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
}

impl TimeOfDay {
    pub fn new(hour: u8, minute: u8) -> Result<Self> {
        ensure!(hour < 24, "Hour must be between 0 and 23");
        ensure!(minute < 60, "Minute must be between 0 and 59");

        Ok(Self { hour, minute })
    }

    pub const fn hour(self) -> u8 {
        self.hour
    }

    pub const fn minute(self) -> u8 {
        self.minute
    }
}

impl FromStr for TimeOfDay {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let (hour, minute) = value
            .trim()
            .split_once(':')
            .context("Time must use HH:MM format")?;

        Self::new(
            hour.parse().context("Invalid hour")?,
            minute.parse().context("Invalid minute")?,
        )
    }
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:02}:{:02}", self.hour, self.minute)
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorldTimeConfig {
    /// Real minutes required for a complete in-game day.
    /// None preserves the speed received from the server.
    pub day_minutes: Option<f64>,

    /// Applied to login time packets.
    pub start_time: Option<TimeOfDay>,

    /// Reapplies start_time to subsequent server time updates.
    /// Does not stop client-side time progression between updates.
    pub pin_on_sync: bool,
}

impl WorldTimeConfig {
    pub fn validate(&self) -> Result<()> {
        self.packet_speed()?;

        ensure!(
            !self.pin_on_sync || self.start_time.is_some(),
            "Pinning requires an explicit start time",
        );

        Ok(())
    }

    pub(super) fn packet_speed(&self) -> Result<Option<f32>> {
        let Some(minutes) = self.day_minutes else {
            return Ok(None);
        };

        ensure!(
            minutes.is_finite() && minutes > 0.0,
            "Day duration must be a finite positive number",
        );

        let speed = (24.0 / minutes) as f32;

        ensure!(
            speed.is_finite() && speed > 0.0,
            "Day duration cannot be represented by the packet speed",
        );

        Ok(Some(speed))
    }
}

#[derive(Debug, Clone, Default)]
pub struct FeatureConfig {
    pub world_time: Option<WorldTimeConfig>,
}

impl FeatureConfig {
    pub fn validate(&self) -> Result<()> {
        if let Some(config) = &self.world_time {
            config.validate()?;
        }

        Ok(())
    }
}
