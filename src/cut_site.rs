use std::rc::Rc;
use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::utils::open_bufreader;

// Contig definition
pub struct Contig {
	pub name: Rc<str>,         	// Contig name
	pub circular: Option<bool>,	// Circular contig flag (None == not circular)
	pub cut_sites: Vec<Site>,		// Vector of sites in numerical order
}

// Cut site definition
#[derive(Debug)]
pub struct Site {
	pub name: String,		// Identifier for cut site
	pub pos: usize,		// Contig position (1 offset)
	pub barcode: String,	// Barcode that matching reads should be assigned to
}

// Collection of cut sites
pub struct CutSites {
	pub chash: HashMap<Rc<str>, Contig>,
}

impl CutSites {
	// Returns cut site closest to position if the distance is <= max_dist, l is the contig length 
	pub fn find_site<S: AsRef<str>>(&self, contig: S, pos: usize, max_dist: usize, l: usize) -> Option<&Site> {
		debug!("Checking for cut site near {}:{}", contig.as_ref(), pos);
		if let Some(ctg) = self.chash.get(contig.as_ref()) {	// Is there a cut site on the contig? 
			// The cut sites are ordered by position for each contig so we can use a binary search
			// This will either return Ok(ix) with the index of the matching position
			// or Err(ix) with the index where the entry should be inserted
			trace!("Match to contig");
			match ctg.cut_sites.binary_search_by_key(&pos, |s| s.pos) {
				// Each match - return corresponding site
				Ok(ix) => {
					trace!("Exact match found: {:?}", ctg.cut_sites[ix]);
					Some(&ctg.cut_sites[ix])
				},
				// No exact match.  Check the two flanking sites (if they exist) and pick the closest
				Err(ix) => {
					let d1 = if ix > 0 {	ctg.cut_sites.get(ix - 1).map(|s| (ix - 1, pos - s.pos)) 
					} else { None };
					let d2 = ctg.cut_sites.get(ix).map(|s| (ix, s.pos - pos));
					if let Some((i, d)) = match(d1, d2) {
						// pos lies between 2 cut sites
						(Some((i, x)), Some((j, y))) => {
							trace!("Possible match between {:?} ({}bp) and {:?} ({}bp)", ctg.cut_sites[i], x, ctg.cut_sites[j], y); 
							if x < y { d1 } else { d2 }
						},
						// pos lies after last cut site
						(Some((i, x)), None) => {
							if ctg.circular.unwrap_or(false) {
								// Check distance to first site on contig
								let x0 = ctg.cut_sites[0].pos;
								let y = if x0 >= pos - l { x0 + l - pos } else { pos - l - x0}; 
								trace!("Possible match between {:?} ({}bp) and {:?} ({}bp)", ctg.cut_sites[i], x, ctg.cut_sites[0], y); 
								if x < y { d1 } else { Some((0, y)) }
							} else { d1 }
						},
						// pos lies before first cut site
						(None, Some((0, y))) => {
							if ctg.circular.unwrap_or(false) {
								// Check distance to last site on contig
								let xn = ctg.cut_sites.last().unwrap().pos;
								let x = if pos >= xn - l { pos + l - xn } else { xn - l - pos};
								trace!("Possible match between {:?} ({}bp) and {:?} ({}bp)", ctg.cut_sites[ctg.cut_sites.len() - 1], x, ctg.cut_sites[0], y); 
								if x < y { Some((ctg.cut_sites.len() - 1, x))} else { d2 }
							} else { d2 }
						},
						// This shouldn't happen
						_ => panic!("Unexpected case!"),
					} {	// Now test if the closest match is closer than max_dist and if so return corresponding element
						if d <= max_dist { 
							trace!("Selected match {:?} ({}bp)", ctg.cut_sites[i], d); 
							Some(&ctg.cut_sites[i]) 
						} else {
							trace!("Unmatched ({}bp)", d); 
							None 
						}
					} else {
						trace!("Unmatched (No candidates)"); 
						None 
					}
				},
			}
		} else { 
			trace!("Unmatched (No candidates)"); 
			None 
		}	// No cut site on contig
	}
}

//  Read in cut site definitions from file
//
//  The cut file should have 4 or 5 tab separated columns:
//    col 1 - contig name
//    col 2 - position in contig (1 offset)
//    col 3 - name of cut site
//    col 4 - sample barcode
//    col 5 - circular flag (true/false yes/no 1/0)
//
//  Returns a CutSites struct
//
pub fn read_cut_file<S: AsRef<Path>>(name: S) -> io::Result<CutSites> {
	let mut chash: HashMap<Rc<str>, Contig> = HashMap::new();
	let mut rdr = open_bufreader(name)?;
	let mut buf = String::new();
	loop {
		let l = rdr.read_line(&mut buf)?;
		if l == 0 { break }
		let fd: Vec<&str> = buf.trim().split('\t').collect();
		if fd.len() > 4 {
			// Get contig from hash or create new entry
			let ctg = if let Some(c) = chash.get_mut(fd[0]) { c } else {
				let name: Rc<str> = Rc::from(fd[0]);
				let c = 	Contig{name: name.clone(), cut_sites: Vec::new(), circular: None};
				chash.insert(name, c);
				chash.get_mut(fd[0]).unwrap()
			};
			// Handle circular flag
			if let Some(fg) = fd.get(4).map(|s| {
				match s.to_lowercase().as_str() {
					"true" | "yes" | "1" => true,
					"false" | "no" | "0" => false,
					_ => panic!("Unknown flag for circular status ({})", s),
				}
			}) {
				if let Some(fg_old) = ctg.circular {
					assert_eq!(fg, fg_old, "Inconsistent circular flag in cut file")
				} else { ctg.circular = Some(fg) } 
				
			}
			// Handle position
			let pos = fd[1].parse::<usize>().expect("Error paring position in cut site file");
			// Create new site
			let site = Site{name: fd[2].to_owned(), barcode: fd[3].to_owned(), pos};
			ctg.cut_sites.push(site);
		}
		buf.clear();
	}	
	// Sort cut_sites by position within each contig
	for (_, ctg) in chash.iter_mut() { ctg.cut_sites.sort_unstable_by_key(|s| s.pos) }
	
	Ok(CutSites{chash})	
}