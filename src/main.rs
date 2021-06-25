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

// Classification of reads from PAF file
#[derive(Debug)]
enum MapResult<'a> {
	Unmapped,							// Unmapped (normally these are not in the file)
	LowMapq,								// Low Mapq (no non-unique mapping records)
	Unmatched,							// No match to a cut site 
	Matched(Match<'a>)	// Match on strand to a cut site 
}

impl <'a>fmt::Display for MapResult<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Unmapped => write!(f, "Unmapped\t*\t*\t*\t*\t*"),
			Self::LowMapq => write!(f, "LowMapQ\t*\t*\t*\t*\t*"),
			Self::Unmatched => write!(f, "Unmatched\t*\t*\t*\t*\t*"),
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
					if let Some(site) = read.find_site(cut_sites, param.mapq_thresh(), param.max_distance()) { MapResult::Matched(site) }
					else { MapResult::Unmatched}
				} else { MapResult::Unmatched }
			} else { MapResult::LowMapq}
		} else { MapResult::Unmapped };
		writeln!(output, "{}\t{}", read.qname(), map_result).map_err(|e| format!("Error writing to output file {}", e))?;
		if let Some(rh) = read_hash.as_mut() { rh.insert(read.qname().to_owned(), map_result); }
	}
	drop(output);
	
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
			let wrt = match rh.get(fq_file.read_id()).unwrap_or(&MapResult::Unmapped) {
				MapResult::Unmapped => &mut ofiles.unmapped,
				MapResult::LowMapq => &mut ofiles.low_mapq,
				MapResult::Unmatched => &mut ofiles.unmatched,
				MapResult::Matched(m) => ofiles.bc_hash.get_mut(m.site.barcode.as_str()).expect("Unknown barcode"),
			};
			fq_file.write_rec(wrt).map_err(|e| format!("Error writing to fastq output: {}", e))?;
		}		
	}

	info!("Done");
		
	Ok(())
}
