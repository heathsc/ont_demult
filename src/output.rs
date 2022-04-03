use std::collections::HashMap;
use std::io::{self, Write, BufWriter};

use compress_io::{
    compress::CompressIo,
    compress_type::CompressType
};

use crate::params::Param;

pub fn open_output_file<S: AsRef<str>>(name: S, param: &Param) -> io::Result<BufWriter<Box<dyn Write>>> {
    let fname = format!("{}_{}", param.prefix(), name.as_ref());
    let mut c = CompressIo::new();
    if param.compress() {
        c.ctype(CompressType::Gzip);
    }
    c.path(fname).bufwriter()
}

pub struct OutputFiles<'a> {
    pub unmapped: Option<BufWriter<Box<dyn Write>>>,
    pub low_mapq: Option<BufWriter<Box<dyn Write>>>,
    pub unmatched: Option<BufWriter<Box<dyn Write>>>,
    pub site_hash: HashMap<&'a str, BufWriter<Box<dyn Write>>>,
}

impl<'a> OutputFiles<'a> {
    pub fn open(param: &'a Param) -> io::Result<OutputFiles<'a>> {
        let (unmapped, low_mapq, unmatched) = if !param.matched_only() {
            (
                Some(open_output_file("unmapped.fastq", param)?),
                Some(open_output_file("low_mapq.fastq", param)?),
                Some(open_output_file("unmatched.fastq", param)?),
            )
        } else {
            (None, None, None)
        };
        let mut site_hash = HashMap::new();
        if let Some(cut_sites) = param.cut_sites() {
            for (_, csites) in cut_sites.chash.iter() {
                for site in csites.cut_sites.iter() {
                    if !site_hash.contains_key(site.name.as_str()) {
                        let wrt = open_output_file(format!("{}.fastq", site.name), param)?;
                        site_hash.insert(site.name.as_str(), wrt);
                    }
                }
            }
        }
        Ok(Self {
            unmapped,
            low_mapq,
            unmatched,
            site_hash,
        })
    }
}
