#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::fmt;

pub mod utils;
pub mod log_level;
pub mod params;
pub mod cut_site;
mod cli;
mod paf;
mod fastq;
mod output;

use params::*;
use paf::*;
use fastq::*;
use output::*;

pub const DEFAULT_PREFIX: &str = "ont_demult";
pub const DEFAULT_MAPQ_THRESHOLD: usize = 10;
pub const DEFAULT_MAX_DISTANCE: usize = 100;
pub const DEFAULT_MARGIN: usize = 5;

// Classification of reads from PAF file
#[derive(Debug)]
enum MapResult<'a> {
	Unmapped(usize),				// Unmapped (normally these are not in the file)
	LowMapq(usize),					// Low Mapq (no non-unique mapping records)
	NoCutSites(usize),				// No cut sites
	Unmatched(Location),		// No match to a cut site 
	Matched(Match<'a>),	// Match on strand to a cut site 
	MatchBoth(Location),
	MatchStart(Location),
	MatchEnd(Location),
	MisMatch(Location),
}

impl <'a>fmt::Display for MapResult<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Unmapped(x) => write!(f, "Unmapped\t*\t*\t*\t*\t*\t*\t*\t{}", x),
			Self::LowMapq(x) => write!(f, "LowMapQ\t*\t*\t*\t*\t*\t*\t*\t{}", x),
			Self::NoCutSites(x) => write!(f, "NoCutSites\t*\t*\t*\t*\t*\t*\t*\t{}", x),
			Self::Unmatched(l) => write!(f, "Unmatched\t{}", l),
			Self::MatchBoth(l) => write!(f, "MatchBoth\t{}", l),
			Self::MatchStart(l) => write!(f, "MatchStart\t{}", l),
			Self::MatchEnd(l) => write!(f, "MatchEnd\t{}", l),
			Self::MisMatch(l) => write!(f, "MisMatch\t{}", l),
			Self::Matched(m) => write!(f, "Matched\t{}", m),
		}
	}
}

fn main() -> Result<(), String> {
	// Process command line arguments
	let param = cli::process_cli().map_err(|e| format!("ont_demult initialization failed with error: {}", e))?;
	
	debug!("Opening PAF input");
	// Open input file (or stdin)
	let mut paf_file = PafFile::open(param.paf_file()).map_err(|e| format!("Error opening paf file: {}", e))?;
	info!("PAF input opened OK");

	// Hash to store read classifications if we will be demultiplexing a FASTQ
	let mut read_hash: Option<HashMap<String, MapResult>> = if param.fastq_file().is_some() { Some(HashMap::new()) } else { None };
	
	// Main output file
	debug!("Opening main output");
	let mut output = open_output_file("res.txt", &param).map_err(|e| format!("Error opening output file: {}", e))?;
	writeln!(output, "read_name\tmatch_status\tcut_site\tbarcode\tstrand\tstart\tlength").map_err(|e| format!("Error writing to output file: {}", e))?;
	// Process PAF reads
	info!("Reading from PAF file");
	while let Some(read) = paf_file.next_read().map_err(|e| format!("Error reading from paf file: {}", e))? {
		let map_result = if read.is_mapped() {
			if read.is_unique(param.mapq_thresh()) {
				if let Some(cut_sites) = param.cut_sites() {
					if let Some(fm) = read.find_site(cut_sites, &param) {
						match fm {
							FindMatch::Match(m) => MapResult::Matched(m), 	
							FindMatch::Location(l) => MapResult::Unmatched(l), 	
							FindMatch::MisMatch(l) => MapResult::MisMatch(l),
							FindMatch::MatchStart(l) => MapResult::MatchStart(l),
							FindMatch::MatchBoth(l) => MapResult::MatchBoth(l),
							FindMatch::MatchEnd(l) => MapResult::MatchEnd(l),
						}
					} else { MapResult::LowMapq(read.qlen)}
				} else { MapResult::NoCutSites(read.qlen) }
			} else { MapResult::LowMapq(read.qlen)}
		} else { MapResult::Unmapped(read.qlen) };
		writeln!(output, "{}\t{}", read.qname(), map_result).map_err(|e| format!("Error writing to output file {}", e))?;
		if let Some(rh) = read_hash.as_mut() { rh.insert(read.qname().to_owned(), map_result); }
	}

	// Process FastQ file if specified
	if let Some(fq) = param.fastq_file() {
		debug!("Opening demultiplexed FastQ output files");
		// Prepare output files
		let mut ofiles = OutputFiles::open(&param).map_err(|e| format!("Error opening FastQ output files: {}", e))?;
		
		// Open input FastQ file
		debug!("Opening FastQ input");
		let mut fq_file = FastqFile::open(fq).map_err(|e| format!("Error opening fastq file: {}", e))?;
		info!("Reading from FastQ file");
		// Process FastQ reads
		let rh = read_hash.as_ref().unwrap();
		while fq_file.next_read().map_err(|e| format!("Error reading from fastq file: {}", e))? {
			let unmapped = MapResult::Unmapped(fq_file.read_len());
			let mr = rh.get(fq_file.read_id()).unwrap_or_else(|| {
				writeln!(output, "{}\t{}", fq_file.read_id(), &unmapped).expect("Error writing to output file {}");
				&unmapped
			});

			if let Some(wrt) = match mr {
				MapResult::Unmapped(_) => ofiles.unmapped.as_mut(),
				MapResult::LowMapq(_) => ofiles.low_mapq.as_mut(),
				MapResult::Matched(m) => ofiles.bc_hash.get_mut(m.site.barcode.as_str()),
				_ => ofiles.unmatched.as_mut(),
			} {
				fq_file.write_rec(wrt).map_err(|e| format!("Error writing to fastq output: {}", e))?
			}
		}		
	}

	info!("Done");
		
	Ok(())
}
