use std::io;

use clap::App;

//mod options;
use super::*;
use crate::log_level::init_log;
use crate::cut_site::read_cut_file;

pub fn process_cli() -> io::Result<Param> {
	let yaml = load_yaml!("cli/cli.yml");
	let app = App::from_yaml(yaml).version(crate_version!());
	
	// Setup logging
	let m = app.get_matches();	
	let _ = init_log(&m);
	
	// Get options
	let prefix = m.value_of("prefix").unwrap_or(DEFAULT_PREFIX);
	let mut params = Param::new(prefix);
	if let Some(file) = m.value_of("fastq") { params.set_fastq_file(file) }
	if let Some(file) = m.value_of("paf_file") { params.set_paf_file(file) }
	if let Some(suffix) = m.value_of("compress_suffix") { params.set_compress_suffix(suffix) }
	if let Some(command) = m.value_of("compress_command") { params.set_compress_command(command) }
	if let Some(sel) = m.value_of("select") { params.set_select(sel) }

	if m.is_present("compress") { params.set_compress() }
	if m.is_present("matched_only") { params.set_matched_only() }
	if params.compress() && params.compress_command().is_none() {
		if params.compress_suffix().is_none() { params.set_compress_suffix("gz") }
		params.set_compress_command("gzip");
	}
	if let Ok(x) = value_t!(m, "maxq_thresh", usize) { params.set_mapq_thresh(x); }
	if let Ok(x) = value_t!(m, "max_distance", usize) { params.set_max_distance(x) }
	if let Ok(x) = value_t!(m, "margin", usize) { params.set_max_distance(x) }
	if let Ok(x) = value_t!(m, "max_unmatched", f64) { params.set_max_unmatched(x) }

	// Process cut file if present
	if let Some(file) = m.value_of("cut_file") {
		params.set_cut_sites(read_cut_file(file)?)
	}
	Ok(params)
}
