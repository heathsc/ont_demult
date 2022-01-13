use std::io::{self, Write};
use std::collections::HashMap;
use std::iter;

use crate::utils::{open_bufwriter, open_pipe_writer};
use crate::params::Param;

pub fn open_output_file<S: AsRef<str>>(name: S, param: &Param) -> io::Result<Box<dyn Write>> {
	let fname = if let Some(suffix) = param.compress_suffix() {
		format!("{}_{}.{}", param.prefix(), name.as_ref(), suffix)
	} else { format!("{}_{}", param.prefix(), name.as_ref()) };

	if let Some(command) = param.compress_command() {
		let fd: Vec<_> = command.split_ascii_whitespace().collect();
		match fd.len() {
			0 => open_bufwriter(&fname),
			1 => open_pipe_writer(&fname, fd[0], iter::empty::<&str>()),
			_ => open_pipe_writer(&fname, fd[0], &fd[1..]),
		}	
	} else { open_bufwriter(&fname) }
} 

pub struct OutputFiles<'a> {
	pub unmapped: Option<Box<dyn Write>>,
	pub low_mapq: Option<Box<dyn Write>>,
	pub unmatched: Option<Box<dyn Write>>,
	pub site_hash: HashMap<&'a str, Box<dyn Write>>,
}

impl <'a>OutputFiles<'a> {
	pub fn open(param: &'a Param) -> io::Result<OutputFiles<'a>> {
		let (unmapped, low_mapq, unmatched) = if !param.matched_only() {
			(Some(open_output_file("unmapped.fastq", param)?),
			Some(open_output_file("low_mapq.fastq", param)?),
			Some(open_output_file("unmatched.fastq", param)?))
		} else { (None, None, None) };
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
		Ok(Self { unmapped, low_mapq, unmatched, site_hash })
	}
}

