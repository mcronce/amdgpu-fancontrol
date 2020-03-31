#![allow(unused_parens)]
#![feature(try_trait)]
#![feature(with_options)]
extern crate envconfig;
#[macro_use]
extern crate envconfig_derive;

use std::time::Duration;
use std::error::Error;

use envconfig::Envconfig;

mod fan_curve;
mod retry_file;

#[derive(Envconfig)]
struct Config {
	#[envconfig(from = "SLEEP_INTERVAL", default = "1")]
	sleep_interval: f32,

	#[envconfig(from = "TEMPS_PWMS", default = "0@65,153@73,255@78")]
	fan_curve: fan_curve::FanCurveConfig,

	#[envconfig(from = "FILE_PWM", default = "/sys/class/drm/card0/device/hwmon/hwmon?/pwm1")]
	file_pwm: String,

	#[envconfig(from = "FILE_FANMODE", default = "/sys/class/drm/card0/device/hwmon/hwmon?/pwm1_enable")]
	file_fanmode: String,

	#[envconfig(from = "FILE_TEMP", default = "/sys/class/drm/card0/device/hwmon/hwmon?/temp1_input")]
	file_temp: String,

	#[envconfig(from = "HYSTERESIS", default = "6")]
	hysteresis: u8,
}

struct State {
	sleep_interval: Duration,
	fan_curve: fan_curve::FanCurve,
	file_pwm: retry_file::RetryFile,
	file_temp: retry_file::RetryFile,
	file_fanmode: retry_file::RetryFile,
	hysteresis: u32,
	temp_at_last_change: u32
}
impl From<Config> for State {
	fn from(config: Config) -> Self {
		Self{
			sleep_interval: Duration::from_secs_f32(config.sleep_interval),
			fan_curve: config.fan_curve.into(),
			file_pwm: retry_file::open_glob_or_panic(&config.file_pwm, 8, true, true),
			file_temp: retry_file::open_glob_or_panic(&config.file_temp, 8, true, false),
			file_fanmode: retry_file::open_glob_or_panic(&config.file_fanmode, 8, false, true),
			hysteresis: (config.hysteresis as u32) * 1000,
			temp_at_last_change: 0
		}
	}
}

fn main() -> Result<(), Box<dyn Error>> {
	let config = Config::init()?;
	let mut state = State::from(config);
	loop {
		let temp = state.file_temp.read_all()?.trim().parse::<u32>()?;
		print!("temp {}\t", temp);
		let current_pwm = state.file_pwm.read_all()?.trim().parse::<u8>()?;
		print!("current PWM {}\t", current_pwm);

		let target_pwm = state.fan_curve.get_target_pwm(temp);
		print!("target PWM {}\n", target_pwm);
		if(target_pwm > current_pwm || (temp < (state.temp_at_last_change - state.hysteresis))) {
			state.file_fanmode.write("1")?;
			state.file_pwm.write(&target_pwm.to_string())?;
			state.temp_at_last_change = temp;
		}
		std::thread::sleep(state.sleep_interval);
	}
}

