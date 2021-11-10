use super::*;
use crate::cut_site::CutSites;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Select {
	Start, Both, Either,
}

impl Select {
	fn from_str(s: &str) -> Option<Self> {
		let s = s.to_ascii_lowercase();
		match s.as_str() {
			"start" => Some(Self::Start),
			"both" => Some(Self::Both),
			"either" => Some(Self::Either),
			_ => None,
		}
	}	
}

// Parameters for run
pub struct Param {
	paf_file: Option<String>,								// Input PAF file (if None, use stdin)
	fastq_file: Option<String>,							// Input FASTQ file (if None, just produce report)
	cut_sites: Option<CutSites>,							// Contigs with cut site definitions (if None, only split based on uniquely mapped/not uniquely mapped)
	prefix: String,											// Ouput prefix (if None, use)
	compress: bool,											// Compress output
	select: Select,											// Selection strategy
	compress_suffix: Option<String>,						// Suffix for compressed files (implies --compress)
	compress_command: Option<String>,					// Command (with arguments) for compression (implies --compress)
	mapq_thresh: usize,										// Minimum threshold for MAPQ
	max_distance: usize,										// Maximum distance allowed from nearest cutsite
}

impl Param {
	pub fn new<S: AsRef<str>>(prefix: S) -> Self {
		let prefix = prefix.as_ref().to_owned();
		Self {
			paf_file: None,
			fastq_file: None,
			cut_sites: None,
			prefix,
			compress: false,
			select: Select::Start,
			compress_suffix: None,
			compress_command: None,
			mapq_thresh: DEFAULT_MAPQ_THRESHOLD,
			max_distance: DEFAULT_MAX_DISTANCE,
		}
	}
	
	// Setters and getters
	pub fn set_paf_file<S: AsRef<str>>(&mut self, paf_file: S) {
		self.paf_file = Some(paf_file.as_ref().to_owned())
	}
	pub fn paf_file(&self) -> Option<&str> { self.paf_file.as_deref() }
	pub fn set_fastq_file<S: AsRef<str>>(&mut self, fastq_file: S) {
		self.fastq_file = Some(fastq_file.as_ref().to_owned())
	}
	pub fn fastq_file(&self) -> Option<&str> { self.fastq_file.as_deref() }
	pub fn set_cut_sites(&mut self, csites: CutSites) {
		self.cut_sites = Some(csites)
	}
	pub fn select(&self) -> Select { self.select }
	pub fn set_select(&mut self, s: &str) { 
		if let Some(sel) = Select::from_str(s) {
			self.select = sel
		}
	}
	pub fn cut_sites(&self) -> Option<&CutSites> { self.cut_sites.as_ref() }
	pub fn prefix(&self) -> &str { &self.prefix }
	pub fn set_compress(&mut self) { self.compress = true }
	pub fn compress(&self) -> bool { self.compress }
	pub fn set_compress_suffix<S: AsRef<str>>(&mut self, com: S) {
		self.compress_suffix = Some(com.as_ref().to_owned());
		self.set_compress();
	}
	pub fn compress_suffix(&self) -> Option<&str> { self.compress_suffix.as_deref() }
	pub fn set_compress_command<S: AsRef<str>>(&mut self, com: S) {
		self.compress_command = Some(com.as_ref().to_owned());
		self.set_compress();
	}
	pub fn compress_command(&self) -> Option<&str> { self.compress_command.as_deref() }
	pub fn set_mapq_thresh(&mut self, m: usize) { self.mapq_thresh = m }
	pub fn mapq_thresh(&self) -> usize { self.mapq_thresh }
	pub fn set_max_distance(&mut self, d: usize) { self.max_distance = d }
	pub fn max_distance(&self) -> usize { self.max_distance }	
}
