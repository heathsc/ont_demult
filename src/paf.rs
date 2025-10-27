// Read and parse Paf file

use std::{
    collections::HashSet,
    fmt,
    io::{self, BufRead, Error},
    path::Path,
    rc::Rc,
    str::FromStr,
};

use compress_io::compress::CompressIo;

use crate::{
    cut_site::{CutSites, Site},
    params::Param,
    strategy::Strategy,
};

fn parse_num<T>(s: &str, msg: &str) -> io::Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: fmt::Debug,
{
    s.parse::<T>()
        .map_err(|e| Error::other(format!("Parse error for {msg}: {e:?}")))
}

// Split line on tabs
fn split(buf: &str, line: usize) -> io::Result<Vec<&str>> {
    let fd: Vec<_> = buf.trim().split('\t').collect();
    if fd.len() < 12 {
        Err(Error::other(format!(
            "Short line (< 12 columns) at line {}",
            line
        )))
    } else {
        Ok(fd)
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Strand {
    Plus,
    Minus,
}

impl fmt::Display for Strand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Plus => '+',
                Self::Minus => '-',
            }
        )
    }
}

#[derive(Debug)]
pub struct Match<'a> {
    pub site: &'a Site,
    inner: CommonLoc,
}

impl<'a> fmt::Display for Match<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\t{}\t{}",
            self.site.name, self.site.barcode, self.inner
        )
    }
}

#[derive(Debug)]
pub struct InteriorSplit {
    from: usize,
    to: usize,
}

#[derive(Debug)]
pub struct Location {
    contig: Rc<str>,
    inner: CommonLoc,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t*\t{}", self.contig, self.inner)
    }
}

#[derive(Debug)]
pub struct CommonLoc {
    strand: Strand,
    start: [usize; 2],
    end: [usize; 2],
    length: usize,
    unused: usize,
    splits: Vec<InteriorSplit>,
}

impl fmt::Display for CommonLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\t{}\t{}\t{}\t{}\t{:.4}",
            self.strand,
            self.start[0],
            self.end[0],
            self.length,
            self.unused,
            (self.unused as f64) / (self.length as f64)
        )?;
        for split in self.splits.iter() {
            write!(f, "\t{}\t{}", split.from, split.to)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum FindMatch<'a> {
    Match(Match<'a>),
    ExcessUnmatched(Match<'a>),
    MisMatch(Location),
    MatchStart(Location),
    MatchBoth(Location),
    MatchEnd(Location),
    Location(Location),
}

impl<'a> fmt::Display for FindMatch<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Match(m) | Self::ExcessUnmatched(m) => write!(f, "{}", m),
            Self::Location(l)
            | Self::MatchBoth(l)
            | Self::MisMatch(l)
            | Self::MatchStart(l)
            | Self::MatchEnd(l) => write!(f, "{}", l),
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
    mapq: u8,
}

impl PafRecord {
    // Make new Paf record from string slice
    // ctgs stores the contigs seen (so we don't have to keep allocating strings to store the name)
    fn from_str_slice(v: &[&str], ctgs: &mut HashSet<Rc<str>>) -> io::Result<Self> {
        assert!(v.len() >= 12);
        let qstart = parse_num(v[2], "query start")?;
        let qend = parse_num(v[3], "query end")?;
        let strand = match v[4] {
            "+" => Strand::Plus,
            "-" => Strand::Minus,
            _ => {
                return Err(Error::other(format!(
                    "Parse error for strand: unrecognized string '{}'",
                    v[4]
                )));
            }
        };
        let target_name = match ctgs.get(v[5]) {
            Some(s) => s.clone(),
            None => {
                let name: Rc<str> = Rc::from(v[5]);
                ctgs.insert(name.clone());
                name
            }
        };
        if qend <= qstart {
            return Err(Error::other(format!(
                "Parse error for {}, query start >= query end",
                target_name
            )));
        }
        let target_length = parse_num(v[6], "target length")?;
        let target_start = parse_num(v[7], "target start")?;
        let target_end = parse_num(v[8], "target end")?;
        let matching_bases = parse_num(v[9], "matching bases")?;
        let mapq = parse_num(v[11], "mapq")?;
        trace!(
            "PAF record {}: {} qstart: {} qend: {} mapq: {}",
            v[0], target_name, qstart, qend, mapq
        );
        Ok(Self {
            qstart,
            qend,
            strand,
            target_name,
            target_length,
            target_start,
            target_end,
            matching_bases,
            mapq,
        })
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
        let qlen = parse_num(v[1], "query length")?;
        let records = vec![PafRecord::from_str_slice(v, ctgs)?];
        if records[0].qend > qlen {
            return Err(Error::other(format!(
                "Parse error for {}, query start > query len",
                qname
            )));
        }
        Ok(Self {
            qname,
            qlen,
            records,
        })
    }
    // Add subsequent records to Paf read
    fn add_record(&mut self, v: &[&str], ctgs: &mut HashSet<Rc<str>>) -> io::Result<()> {
        assert!(v.len() >= 12);
        assert_eq!(self.qname, v[0]);
        let rec = PafRecord::from_str_slice(v, ctgs)?;
        if rec.qend > self.qlen {
            return Err(Error::other(format!(
                "Parse error for {}, query start > query len",
                self.qname
            )));
        }
        self.records.push(rec);
        Ok(())
    }
    pub fn qname(&self) -> &str {
        &self.qname
    }
    // Check if read is mapped
    pub fn is_mapped(&self) -> bool {
        self.records.iter().all(|r| r.target_name.as_ref() != "*")
    }
    // Check if read has one mapping with mapq >= threshold
    pub fn is_unique(&self, threshold: u8) -> bool {
        self.records.iter().any(|r| r.mapq >= threshold)
    }
    // Check for match to cut-site
    // Strategy - look for mapping records that can be assembled to cover more or less
    // the whole read where at least 1 record has a mapq > threshold and the others are on
    // the same contig strand
    pub fn find_site<'b>(&self, cut_sites: &'b CutSites, param: &Param) -> Option<FindMatch<'b>> {
        debug!("Checking matches for read {}", self.qname);
        let threshold = param.mapq_thresh();
        let max_dist = param.max_distance();
        let select = param.select();
        let margin = param.margin();

        // Find longest uniquely mapping record, filtering out reads much longer than the reference
        self.records
            .iter()
            .filter(|r| r.mapq >= threshold && self.qlen < r.target_length + 150)
            .max_by_key(|r| r.matching_bases)
            .and_then(|r| {
                trace!(
                    "Found longest match: query: {} {} {} {} target: {} {} {}",
                    self.qlen,
                    r.qstart,
                    r.qend,
                    r.strand,
                    r.target_name,
                    r.target_start,
                    r.target_end
                );

                let strand = r.strand;

                // Select other records on same contig strand as longest match with mapq > 0
                let mut recs: Vec<_> = self
                    .records
                    .iter()
                    .filter(|s| {
                        s.target_name == r.target_name && s.strand == r.strand && s.mapq > 0
                    })
                    .collect();

                recs.sort_unstable_by_key(|s| s.qstart);

                // Find record that starts earliest in the read
                let s = &recs[0];
                trace!(
                    "First record in read - query: {} {} {} {} target: {} {}",
                    self.qlen, s.qstart, s.qend, s.strand, s.target_start, s.target_end
                );

                let mut skip = false;
                // Check for overlaps in read between records
                for s in recs.windows(2) {
                    if s[0].qend >= s[1].qstart {
                        trace!(
                            "Read {} mapping to {} overlaps by {} bases - discarded",
                            self.qname,
                            r.target_name,
                            s[0].qend - s[1].qstart + 1
                        );
                        skip = true;
                        break;
                    }
                }

                // check for reads with large unused portions
                let unused = if !skip {
                    let mut used = 0;
                    for s in recs.iter() {
                        used += s.qend - s.qstart;
                    }
                    assert!(used <= self.qlen);
                    self.qlen - used
                } else {
                    0
                };

                if !skip {
                    // Increase starting position by margin to allow for 'overrun'
                    let (start, spos) = match s.strand {
                        Strand::Plus => (s.target_start, s.target_start + margin),
                        Strand::Minus => (s.target_end, s.target_end.saturating_sub(margin)),
                    };
                    trace!("Using starting position {}", spos);

                    // Find record that ends latest in read
                    let s1 = recs.iter().max_by_key(|s| s.qend).unwrap();

                    // Increase starting position and reduce ending position by margin to allow for 'overrun'

                    let (end, send) = match s1.strand {
                        Strand::Plus => (s1.target_end, s1.target_end.saturating_sub(margin)),
                        Strand::Minus => (s1.target_start, s1.target_start + margin),
                    };

                    trace!("Using ending position {}", send);
                    // Look for matching cut site
                    let start_site = cut_sites.find_site(
                        s.target_name.as_ref(),
                        spos,
                        strand == Strand::Plus,
                        max_dist,
                        s.target_length,
                    );
                    let end_site = cut_sites.find_site(
                        s.target_name.as_ref(),
                        send,
                        strand == Strand::Minus,
                        max_dist,
                        s.target_length,
                    );
                    trace!("start_site: {:?}, end_site: {:?}", start_site, end_site);

                    // Get splits
                    let splits: Vec<_> = recs
                        .windows(2)
                        .map(|x| {
                            if strand == Strand::Plus {
                                InteriorSplit {
                                    from: x[0].target_end,
                                    to: x[1].target_start,
                                }
                            } else {
                                InteriorSplit {
                                    from: x[0].target_start,
                                    to: x[1].target_end,
                                }
                            }
                        })
                        .collect();

                    let cloc = CommonLoc {
                        strand: s.strand,
                        start: [start, spos],
                        end: [end, send],
                        length: self.qlen,
                        unused,
                        splits,
                    };
                    let check_match = |m| {
                        if unused > param.max_unmatched() {
                            FindMatch::ExcessUnmatched(m)
                        } else {
                            FindMatch::Match(m)
                        }
                    };

                    Some(match (start_site, end_site, select) {
                        (Some(m1), Some(m2), sel) => {
                            if m1 == m2 {
                                if sel == Strategy::Xor {
                                    FindMatch::MatchBoth(Location {
                                        contig: s.target_name.clone(),
                                        inner: cloc,
                                    })
                                } else {
                                    check_match(Match {
                                        site: m1,
                                        inner: cloc,
                                    })
                                }
                            } else {
                                FindMatch::MisMatch(Location {
                                    contig: s.target_name.clone(),
                                    inner: cloc,
                                })
                            }
                        }
                        (Some(_), None, Strategy::Both) => FindMatch::MatchStart(Location {
                            contig: s.target_name.clone(),
                            inner: cloc,
                        }),
                        (Some(m), None, _) => check_match(Match {
                            site: m,
                            inner: cloc,
                        }),
                        (None, Some(m), Strategy::Either) | (None, Some(m), Strategy::Xor) => {
                            check_match(Match {
                                site: m,
                                inner: cloc,
                            })
                        }
                        (None, Some(_), _) => FindMatch::MatchEnd(Location {
                            contig: s.target_name.clone(),
                            inner: cloc,
                        }),
                        (None, None, _) => FindMatch::Location(Location {
                            contig: s.target_name.clone(),
                            inner: cloc,
                        }),
                    })
                } else {
                    None
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
            rdr: CompressIo::new().opt_path(name).bufreader().map(Box::new)?,
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
        if self.eof {
            return Ok(None);
        }
        // Read next line if not already in buf
        if self.buf.is_empty() && self.next_line()? == 0 {
            return Ok(None);
        }
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
            } else {
                break;
            }
        }
        Ok(Some(paf_read))
    }
}
