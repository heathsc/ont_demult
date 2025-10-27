use super::{strategy::Strategy, DEFAULT_PREFIX};
use crate::cut_site::CutSites;

#[derive(Debug, Default)]
pub struct ParamBuilder {
    paf_file: Option<String>,
    fastq_file: Option<String>,
    cut_sites: Option<CutSites>,
    prefix: Option<String>,
    compress: bool,
    matched_only: bool,
    select: Strategy,
    mapq_thresh: u8,
    max_distance: usize,
    max_unmatched: usize,
    margin: usize,
}

impl ParamBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn build(self) -> Param {
        Param {
            paf_file: self.paf_file,
            fastq_file: self.fastq_file,
            cut_sites: self.cut_sites,
            prefix: self.prefix.unwrap_or(DEFAULT_PREFIX.to_string()),
            compress: self.compress,
            matched_only: self.matched_only,
            select: self.select,
            mapq_thresh: self.mapq_thresh,
            max_distance: self.max_distance,
            max_unmatched: self.max_unmatched,
            margin: self.margin,
        }
    }

    pub fn paf_file<S: AsRef<str>>(&mut self, file: S) -> &mut Self {
        self.paf_file = Some(file.as_ref().to_owned());
        self
    }

    pub fn fastq_file<S: AsRef<str>>(&mut self, file: S) -> &mut Self {
        self.fastq_file = Some(file.as_ref().to_owned());
        self
    }

    pub fn cut_sites(&mut self, csites: CutSites) -> &mut Self {
        self.cut_sites = Some(csites);
        self
    }

    pub fn select(&mut self, select: Strategy) -> &mut Self {
        self.select = select;
        self
    }

    pub fn prefix<S: AsRef<str>>(&mut self, prefix: S) -> &mut Self {
        self.prefix = Some(prefix.as_ref().to_owned());
        self
    }

    pub fn compress(&mut self, yes: bool) -> &mut Self {
        self.compress = yes;
        self
    }

    pub fn matched_only(&mut self, yes: bool) -> &mut Self {
        self.matched_only = yes;
        self
    }

    pub fn mapq_thresh(&mut self, x: u8) -> &mut Self {
        self.mapq_thresh = x;
        self
    }

    pub fn max_distance(&mut self, x: usize) -> &mut Self {
        self.max_distance = x;
        self
    }

    pub fn max_unmatched(&mut self, x: usize) -> &mut Self {
        self.max_unmatched = x;
        self
    }

    pub fn margin(&mut self, x: usize) -> &mut Self {
        self.margin = x;
        self
    }
}

// Parameters for run
#[derive(Debug, Default)]
pub struct Param {
    paf_file: Option<String>,         // Input PAF file (if None, use stdin)
    fastq_file: Option<String>,       // Input FASTQ file (if None, just produce report)
    cut_sites: Option<CutSites>, // Contigs with cut site definitions (if None, only split based on uniquely mapped/not uniquely mapped)
    prefix: String,              // Output prefix (if None, use)
    compress: bool,              // Compress output
    matched_only: bool,          // Only output matched fastq records when demultiplexing
    select: Strategy,              // Selection strategy
    mapq_thresh: u8,               // Minimum threshold for MAPQ
    max_distance: usize,              // Maximum distance allowed from nearest cut site
    max_unmatched: usize, // Maximum proportion number of unmatched bases allowed per read
    margin: usize,        // Extra margin allowed when matching on 'wrong side' of cut site
}

impl Param {
    pub fn paf_file(&self) -> Option<&str> {
        self.paf_file.as_deref()
    }
    pub fn fastq_file(&self) -> Option<&str> {
        self.fastq_file.as_deref()
    }
    pub fn select(&self) -> Strategy {
        self.select
    }
    pub fn cut_sites(&self) -> Option<&CutSites> {
        self.cut_sites.as_ref()
    }
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
    pub fn compress(&self) -> bool {
        self.compress
    }
    pub fn matched_only(&self) -> bool {
        self.matched_only
    }
    pub fn mapq_thresh(&self) -> u8 {
        self.mapq_thresh
    }
    pub fn max_distance(&self) -> usize {
        self.max_distance
    }
    pub fn margin(&self) -> usize {
        self.margin
    }
    pub fn max_unmatched(&self) -> usize {
        self.max_unmatched
    }
}
