extern crate glob;

use std::error;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

#[derive(Debug, Clone)]
pub struct IoError<T: fmt::Debug> {
	inner: T
}
impl<T: fmt::Debug> IoError<T> {
	fn new(inner: T) -> Self {
		Self{inner: inner}
	}
}
impl<T: fmt::Debug> fmt::Display for IoError<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.inner.fmt(f)
	}
}
impl<T: fmt::Debug> error::Error for IoError<T> {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		None
	}
}

pub struct RetryFile {
	path: String,
	retry_count: u8,
	retry_max: u8,
	read: bool,
	write: bool,
	fd: File,
}

pub fn open(path: &str, retry_max: u8, read: bool, write: bool) -> Result<RetryFile, std::io::Error> {
	Ok(RetryFile{
		path: path.to_string(),
		retry_count: 0,
		retry_max: retry_max,
		read: read,
		write: write,
		fd: File::with_options().read(read).write(write).open(path)?
	})
}

pub fn open_glob(path: &str, retry_max: u8, read: bool, write: bool) -> Result<RetryFile, Box<dyn error::Error>> {
	println!("glob: {}", path);
	let mut files = glob::glob(&path)?;
	let first_file = match files.next() {
		Some(v) => v?,
		None => return Err(Box::new(IoError::new(std::option::NoneError)))
	};
	let first_file_str = match first_file.to_str() {
		Some(v) => v,
		None => return Err(Box::new(IoError::new(std::option::NoneError)))
	};
	println!(" -> file: {}", first_file_str);
	match open(first_file_str, retry_max, read, write) {
		Ok(v) => Ok(v),
		Err(e) => Err(Box::new(IoError::new(e)))
	}
}

pub fn open_glob_or_panic(path: &str, retry_max: u8, read: bool, write: bool) -> RetryFile {
	match open_glob(path, retry_max, read, write) {
		Ok(v) => v,
		Err(e) => panic!(format!("{}", e))
	}
}

// TODO:  Replace read/write functions with std::io::Read and std::io::Write
//    impls and maybe this makes a good standalone crate
impl RetryFile {
	pub fn reopen(&mut self) -> Result<(), std::io::Error> {
		self.fd = match File::with_options().read(self.read).write(self.write).open(&self.path) {
			Ok(v) => v,
			Err(e) => {
				self.retry_count += 1;
				if(self.retry_count > self.retry_max) {
					return Err(e);
				}
				return self.reopen()
			}
		};
		Ok(())
	}

	pub fn rewind(&mut self) -> Result<u64, std::io::Error> {
		self.fd.seek(SeekFrom::Start(0))
	}

	pub fn read_to_end(&mut self) -> Result<String, std::io::Error> {
		let mut contents = String::new();
		loop {
			self.retry_count += 1;
			match self.fd.read_to_string(&mut contents) {
				Ok(_) => {
					self.retry_count = 0;
					return Ok(contents)
				},
				Err(e) => {
					println!("!!! read_to_end(): {}", e);
					self.reopen()?
				}
			};
		};
	}

	pub fn read_all(&mut self) -> Result<String, std::io::Error> {
		self.rewind()?;
		let result = self.read_to_end()?;
		Ok(result)
	}

	pub fn write(&mut self, buf: &str) -> Result<usize, std::io::Error> {
		let bytes = buf.as_bytes();
		let written = loop {
			self.retry_count += 1;
			match self.fd.write(bytes) {
				Ok(v) => {
					match self.fd.flush() {
						Ok(_) => {
							self.retry_count = 0;
							break v;
						}
						Err(_) => self.reopen()?
					}
				},
				Err(_) => self.reopen()?
			};
		};
		Ok(written)
	}
}

