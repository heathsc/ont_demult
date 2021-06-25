// Read and parse FASTQ file

use std::io::{self, BufRead, Write, Error, ErrorKind};
use std::path::Path;

use crate::utils::open_bufreader;

fn gen_err(s: &str, line: usize) -> io::Error {
	Error::new(ErrorKind::Other, format!("{} at line {}", s, line))
}

pub struct FastqFile {
	rdr: Box<dyn BufRead>,
	buf: [String; 3],
	line: usize,
}

impl FastqFile {
	pub fn open<P: AsRef<Path>>(name: P) -> io::Result<Self> {
		Ok(Self {
			rdr: open_bufreader(name)?,
			buf: [String::new(), String::new(), String::new()],
			line: 0,
		})	
	}
	
	// Get next line from fastq file
	fn next_line(&mut self, ix: usize) -> io::Result<usize> {
		self.buf[ix].clear();
		self.line += 1;
		self.rdr.read_line(&mut self.buf[ix])
	}
	
	// Get next read from fastq file (i.e., the id, seq and qual lines)
	// Returns Err on failure, Ok(false) on EOF and Ok(true) on success
	pub fn next_read(&mut self) -> io::Result<bool> {
		// Get line with read tag
		if self.next_line(0)? == 0 { return Ok(false) }
		if !self.buf[0].starts_with('@') { return Err(gen_err("Unexpected character (expected '@' at start of line)", self.line))}
		// Get sequence line
		if self.next_line(1)? == 0 { return Err(gen_err("Incomplete record", self.line)) }
		// Get line 3 (just check for initial '+')
		if self.next_line(2)? == 0 { return Err(gen_err("Incomplete record", self.line)) }
		if !self.buf[2].starts_with('+') { return Err(gen_err("Unexpected character (expected '+' at start of line)", self.line))}
		// Get quality line
		if self.next_line(2)? == 0 { return Err(gen_err("Incomplete record", self.line)) }
		if self.buf[1].len() != self.buf[2].len() { return Err(gen_err("Sequence and quality lines are different lengths", self.line)) }
		Ok(true)
	}
	
	// Returns read_id
	pub fn read_id(&self) -> &str {
		// Removes initial '@' and splits on first white space character (or returns whole line if not present)
		let tag = self.buf[0][1..].split_once(char::is_whitespace).map(|(a, _)| a).unwrap_or(&self.buf[0]);
		// Remove end tag if present
		match tag.rsplit_once('/') {
			Some((a, "1" | "2")) => a,
			_ => tag,
		}
	}
	
	pub fn write_rec(&self, wrt: &mut Box<dyn Write>) -> io::Result<()> {
		write!(wrt, "{}{}+\n{}", self.buf[0], self.buf[1], self.buf[2])
	}
}
