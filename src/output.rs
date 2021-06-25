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
	pub unmapped: Box<dyn Write>,	
	pub low_mapq: Box<dyn Write>,	
	pub unmatched: Box<dyn Write>,	
	pub bc_hash: HashMap<&'a str, Box<dyn Write>>, 
}

impl <'a>OutputFiles<'a> {
	pub fn open(param: &'a Param) -> io::Result<OutputFiles<'a>> {
		let unmapped = open_output_file("unmapped.fastq", param)?;
		let low_mapq = open_output_file("low_mapq.fastq", param)?;
		let unmatched = open_output_file("unmatched.fastq", param)?;
		let mut bc_hash = HashMap::new();
		if let Some(cut_sites) = param.cut_sites() {
			for (_, csites) in cut_sites.chash.iter() {
				for site in csites.cut_sites.iter() {
					if !bc_hash.contains_key(site.barcode.as_str()) {
						let wrt = open_output_file(format!("{}.fastq", site.barcode), param)?;
						bc_hash.insert(site.barcode.as_str(), wrt);
					}
				}
			}	
		}
		Ok(Self { unmapped, low_mapq, unmatched, bc_hash })
	}
}

