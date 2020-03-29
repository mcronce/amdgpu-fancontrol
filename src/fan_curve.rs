use std::collections::BTreeMap;
use std::collections::btree_map::Iter;
use std::convert::TryInto;
use std::error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct FanCurveParseError /* {{{ */ {
	bad_string: String
}
impl FanCurveParseError {
	fn new(bad_string: &str) -> Self {
		Self{bad_string: bad_string.to_string()}
	}
}
impl fmt::Display for FanCurveParseError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "failed to parse {} as a fan curve", self.bad_string)
	}
}
impl error::Error for FanCurveParseError {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		None
	}
}
// }}}

type Result<T> = std::result::Result<T, FanCurveParseError>;

pub struct FanCurveConfig(BTreeMap<u8, u8>); // {{{
impl FanCurveConfig {
	fn new() -> Self {
		Self(BTreeMap::new())
	}

	fn insert(&mut self, temp: u8, pwm: u8) -> Option<u8> {
		self.0.insert(temp, pwm)
	}

	fn iter(&self) -> Iter<u8, u8> {
		self.0.iter()
	}
}
impl FromStr for FanCurveConfig {
	type Err = FanCurveParseError;
	fn from_str(s: &str) -> Result<Self> {
		if(s.len() == 0) {
			return Err(FanCurveParseError::new(s));
		}
		let points = s.split(",");
		let mut this = FanCurveConfig::new();
		for point in points {
			let point = point.split("@").collect::<Vec<&str>>();
			if(point.len() != 2) {
				return Err(FanCurveParseError::new(s));
			}
			let temp = match point[1].parse::<u8>() {
				Ok(v) => v,
				Err(_) => return Err(FanCurveParseError::new(s))
			};
			let pwm = match point[0].parse::<u8>() {
				Ok(v) => v,
				Err(_) => return Err(FanCurveParseError::new(s))
			};
			this.insert(temp, pwm);
		}
		Ok(this)
	}
}
// }}}

pub struct FanCurve(Vec<(u32, u8)>);
impl FanCurve {
	pub fn get_target_pwm(&self, current_temp: u32) -> u8 {
		let last = self.0.len() - 1;
		if(last == 0) {
			panic!("FanCurve::get_target_pwm():  The fan curve requires at least two entries");
		}

		if(current_temp <= self.0[0].0) {
			// The temperature is below the first entry in the fan curve; set the fan speed to minimum
			return self.0[0].1;
		} else if(current_temp >= self.0[last].0) {
			// The temperature is above the last entry in the fan curve; set the fan speed to maximum
			return self.0[last].1;
		}

		// TODO:  Benchmark doing this with iterators so we can avoid bounds
		//    checks, it might be complicated enough to be slower than this or
		//    might be a solid improvement.
		for i in 1..self.0.len() {
			let higher = self.0[i];
			if(current_temp > higher.0) {
				continue;
			}
			let lower = self.0[i - 1];
			// Linear interpolation
			let temp_offset = current_temp - lower.0;
			let pwm_difference = (higher.1 - lower.1) as u32;
			let temp_difference = higher.0 - lower.0;
			return ((temp_offset * pwm_difference / temp_difference) + lower.1 as u32).try_into().expect("somehow interpolated a target PWM greater than 255");
		}
		// Should be unreachable
		self.0[last].1
	}
}
impl From<FanCurveConfig> for FanCurve {
	fn from(config: FanCurveConfig) -> Self {
		FanCurve(config.iter().map(|(temp, pwm)| (*temp as u32 * 1000, *pwm)).collect())
	}
}

