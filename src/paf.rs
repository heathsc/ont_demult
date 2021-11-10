// Read and parse Paf file

use std::io::{self, BufRead, Error, ErrorKind};
use std::path::Path;
use std::rc::Rc;
use std::collections::HashSet;
use std::fmt;

use crate::utils::get_reader;
use crate::cut_site::{Site, CutSites};
use crate::params::Select;

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
	end: usize,
	length: usize,
}

impl <'a>fmt::Display for Match<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}\t{}\t{}\t{}\t{}\t{}", self.site.name, self.site.barcode, self.strand, self.start, self.end, self.length)
	}
}

#[derive(Debug)]
pub struct Location {
	contig: Rc<str>,
	strand: Strand,
	start: usize,
	end: usize,
	length: usize,
}

impl fmt::Display for Location {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}\t*\t{}\t{}\t{}\t{}", self.contig, self.strand, self.start, self.end, self.length)
	}
}

#[derive(Debug)]
pub enum FindMatch<'a> {
	Match(Match<'a>),
	MisMatch(Location),
	MatchStart(Location),
	MatchEnd(Location),
	Location(Location),
}

impl <'a>fmt::Display for FindMatch<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Match(m) => write!(f, "{}", m),
			Self::Location(l) | Self::MisMatch(l) | Self::MatchStart(l) | Self::MatchEnd(l)  => write!(f, "{}", l),
		}
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
	pub qlen: usize,
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
	pub fn find_site<'a, 'b>(&'a self, cut_sites: &'b CutSites, threshold: usize, max_dist: usize, select: Select) -> Option<FindMatch<'b>> {
		debug!("Checking matches for read {}", self.qname);
		// Find longest uniquely mapping record, filtering out reads much longer than the reference
		self.records.iter().filter(|r| r.mapq >= threshold && self.qlen < r.target_length + 150).max_by_key(|r| r.matching_bases).map(|r| {
			trace!("Found longest match: query: {} {} {} {} target: {} {} {}", 
				self.qlen, r.qstart, r.qend, r.strand, r.target_name, r.target_start, r.target_end);
			// Select other records on same contig strand as longest match with mapq > 0
			let recs: Vec<_> = self.records.iter().filter(|s| s.target_name == r.target_name && s.strand == r.strand && s.mapq > 0).collect();
			
			// Find record that starts earliest in the read
			let s = recs.iter().min_by_key(|s| s.qstart).unwrap();
			trace!("First record in read - query: {} {} {} {} target: {} {}", self.qlen, s.qstart, s.qend, s.strand, s.target_start, s.target_end);
			// Infer true starting position by adjusting for unmatched bases
			let spos = match s.strand {
				Strand::Plus => if s.qstart <= s.target_start { s.target_start - s.qstart } else { 0 },
				Strand::Minus => s.target_end + s.qstart,
			};
			trace!("Using starting position {}", spos);
			
			// Find record that ends latest in read
			let s1 = recs.iter().max_by_key(|s| s.qend).unwrap();
			
			// Infer true ending position by adjusting for unmatched bases
			let unmatched = self.qlen - s1.qend;
			let send = match s1.strand {
				Strand::Minus => if unmatched <= s1.target_start { s1.target_start - unmatched } else { 0 },
				Strand::Plus => s1.target_end + unmatched,
			};
			
			trace!("Using ending position {}", send);
			// Look for matching cut site
			let start_site = cut_sites.find_site(s.target_name.as_ref(), spos, max_dist, s.target_length);
			let end_site = cut_sites.find_site(s.target_name.as_ref(), send, max_dist, s.target_length);
			trace!("start_site: {:?}, end_site: {:?}", start_site, end_site);
			
			match (start_site, end_site, select) {
				(Some(m1), Some(m2), _) => if m1 == m2 {
					FindMatch::Match(Match{site: m1, strand: s.strand, start: spos, end: send, length: self.qlen})
				} else {
					FindMatch::MisMatch(Location{contig: s.target_name.clone(), strand: s.strand, start: spos, end: send, length: self.qlen})
				},
				(Some(_), None, Select::Both) => FindMatch::MatchStart(Location{contig: s.target_name.clone(), strand: s.strand, start: spos, end: send, length: self.qlen}),
				(Some(m), None, _) => FindMatch::Match(Match{site: m, strand: s.strand, start: spos, end: send, length: self.qlen}),
				(None, Some(m), Select::Either) => FindMatch::Match(Match{site: m, strand: s.strand, start: spos, end: send, length: self.qlen}),
				(None, Some(_), _) => FindMatch::MatchEnd(Location{contig: s.target_name.clone(), strand: s.strand, start: spos, end: send, length: self.qlen}),
				(None, None, _) => FindMatch::Location(Location{contig: s.target_name.clone(), strand: s.strand, start: spos, end: send, length: self.qlen}),
			}
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