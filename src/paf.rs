// Read and parse Paf file

use std::io::{self, BufRead, Error, ErrorKind};
use std::path::Path;
use std::rc::Rc;
use std::collections::HashSet;
use std::fmt;

use crate::utils::get_reader;
use crate::cut_site::{Site, CutSites};

fn parse_usize(s: &str, msg: &str) -> io::Result<usize> {	
	s.parse::<usize>().map_err(|e| Error::new(ErrorKind::Other, format!("Parse error for {}: {}", msg, e)))
}

// Split line on tabs
fn split(buf: &str, line: usize) -> io::Result<Vec<&str>> {
	let fd: Vec<_> = buf.trim().split('\t').collect();
	if fd.len() < 12 {
		Err(Error::new(ErrorKind::Other, format!("Short line (< 12 columns) at line {}", line)))
	} else {
		Ok(fd)
	}	
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Strand { Plus, Minus }

impl fmt::Display for Strand {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", match self {
			Self::Plus => '+',
			Self::Minus => '-',
		})
	}
}

#[derive(Debug)]
pub struct Match<'a> {
	pub site: &'a Site,
	strand: Strand,
	start: usize,
	length: usize,
}

impl <'a>fmt::Display for Match<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}\t{}\t{}\t{}\t{}", self.site.name, self.site.barcode, self.strand, self.start, self.length)
	}
}

pub struct PafRecord {
	qstart: usize,
	qend: usize,
	strand: Strand,
	target_name: Rc<str>,
	target_length: usize,
	target_start: usize,
	target_end: usize,
	matching_bases: usize,
	mapq: usize,
}

impl PafRecord {
	// Make new Paf record from string slice
	// ctgs stores the contigs seen (so we don't have to keep allocating strings to store the name)
	fn from_str_slice(v: &[&str], ctgs: &mut HashSet<Rc<str>>) -> io::Result<Self> {
		assert!(v.len() >= 12);
		let qstart = parse_usize(v[2], "query start")?;
		let qend = parse_usize(v[3], "query start")?;
		let strand = match v[4] {
			"+" => Strand::Plus,
			"-" => Strand::Minus,
			_ => return Err(Error::new(ErrorKind::Other, format!("Parse error for strand: unrecognized string '{}'", v[4]))),
		};
		let target_name = match ctgs.get(v[5]) {
			Some(s) => s.clone(),
			None => {
				let name: Rc<str> = Rc::from(v[5]);
				ctgs.insert(name.clone());
				name
			},
		};
		let target_length = parse_usize(v[6], "target length")?;
		let target_start = parse_usize(v[7], "target start")?;
		let target_end = parse_usize(v[8], "target end")?;
		let matching_bases = parse_usize(v[9], "matching bases")?;
		let mapq = parse_usize(v[11], "mapq")?;
		Ok(Self{qstart, qend, strand, target_name, target_length, target_start, target_end, matching_bases, mapq})
	}	
}

pub struct PafRead {
	qname: String,
	qlen: usize,
	records: Vec<PafRecord>,	
}

impl PafRead {
	// Make new Paf read from string slice with first mapping record
	// ctgs stores the contigs seen (so we don't have to keep allocating strings to store the name)
	fn from_str_slice(v: &[&str], ctgs: &mut HashSet<Rc<str>>) -> io::Result<Self> {
		assert!(v.len() >= 12);
		let qname = v[0].to_owned();
		let qlen = parse_usize(v[1], "query length")?;
		let records = vec!(PafRecord::from_str_slice(v, ctgs)?);
		Ok(Self{qname, qlen, records})
	}	
	// Add subsequent records to Paf read
	fn add_record(&mut self, v: &[&str], ctgs: &mut HashSet<Rc<str>>) -> io::Result<()> {
		assert!(v.len() >= 12);
		assert!(self.qname == v[0]);
		self.records.push(PafRecord::from_str_slice(v, ctgs)?);
		Ok(())
	}
	pub fn qname(&self) -> &str { &self.qname }
	// Check if read is mapped
	pub fn is_mapped(&self) -> bool {
		self.records.iter().all(|r| r.target_name.as_ref() != "*")
	}
	// Check if read has one mapping with mapq >= threshold
	pub fn is_unique(&self, threshold: usize) -> bool {
		self.records.iter().any(|r| r.mapq >= threshold)
	}
	// Check for match to cut-site
	// Strategy - look for mapping records that can be assembled to cover more or less
	// the whole read where at least 1 record has a mapq > threshold and the others are on
	// the same contig strand
	pub fn find_site<'a, 'b>(&'a self, cut_sites: &'b CutSites, threshold: usize, max_dist: usize) -> Option<Match<'b>> {
		debug!("Checking matches for read {}", self.qname);
		// Find longest uniquely mapping record
		self.records.iter().filter(|r| r.mapq >= threshold).max_by_key(|r| r.matching_bases).and_then(|r| {
			trace!("Found longest match: query: {} {} {} {} target: {} {} {}", 
				self.qlen, r.qstart, r.qend, r.strand, r.target_name, r.target_start, r.target_end);
			// Select other records on same contig strand as longest match with mapq > 0
			let recs: Vec<_> = self.records.iter().filter(|s| s.target_name == r.target_name && s.strand == r.strand && s.mapq > 0).collect();
			let s = recs.iter().min_by_key(|s| s.qstart).unwrap();
			trace!("First record in read - query: {} {} {} {} target: {} {}", self.qlen, s.qstart, s.qend, s.strand, s.target_start, s.target_end);
			// Infer true starting position by adjusting for unmatched bases
			let spos = match s.strand {
				Strand::Plus => if s.qstart <= s.target_start { s.target_start - s.qstart } else { 0 },
				Strand::Minus => s.target_end + s.qstart,
			};
			trace!("Using starting position {}", spos);
			// Look for matching cut site
			cut_sites.find_site(s.target_name.as_ref(), spos, max_dist, s.target_length)
				.map(|site| Match{site, strand: s.strand, start: spos, length: self.qlen})
		})
	}
}

pub struct PafFile {
	rdr: Box<dyn BufRead>,
	buf: String,
	ctgs: HashSet<Rc<str>>, 
	line: usize,
	eof: bool,
}

impl PafFile {
	pub fn open<P: AsRef<Path>>(name: Option<P>) -> io::Result<Self> {
		Ok(Self {
			rdr: get_reader(name)?,
			buf: String::new(),
			ctgs: HashSet::new(),
			line: 0,
			eof: false,
		})	
	}
	// Get next line from paf file
	fn next_line(&mut self) -> io::Result<usize> {
		self.buf.clear();
		self.line += 1;
		self.rdr.read_line(&mut self.buf)
	}
	// Get next read from paf file (i.e., all mapping records corresponding to a read)
	pub fn next_read(&mut self) -> io::Result<Option<PafRead>> {
		if self.eof { return Ok(None) }
		// Read next line if not already in buf
		if self.buf.is_empty() && self.next_line()? == 0 { return Ok(None) }	
		// Split on tabs
		let fd = split(&self.buf, self.line)?;
		// Parse first mapping record
		let mut paf_read = PafRead::from_str_slice(&fd, &mut self.ctgs)?;
		// Add additional reads
		loop {
			if self.next_line()? == 0 {
				self.eof = true;
				break;
			}
			// Split on tabs
			let fd = split(&self.buf, self.line)?;
			if fd[0] == paf_read.qname {
				paf_read.add_record(&fd, &mut self.ctgs)?;
			} else { break }
		}
		Ok(Some(paf_read))
	}
}