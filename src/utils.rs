use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufRead, BufWriter, Error, ErrorKind, Result, stdin, stdout};
use std::process::{Command, Stdio, ChildStdout, ChildStdin};
use std::path::{Path, PathBuf};
use std::ffi::{OsString, OsStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::env;

// Check if file at path p is accessible and executable by user
fn access(p: &Path) -> Result<bool> {
	let cstr = CString::new(p.as_os_str().as_bytes()).map_err(|e| Error::new(ErrorKind::Other, format!("access(): error converting {}: {}", p.display(), e)))?;
	unsafe { Ok(libc::access(cstr.as_ptr(), libc::X_OK) == 0) }
}

// Search for executable "prog" in PATH 
fn find_exec_path<S: AsRef<OsStr>>(prog: S) -> Option<PathBuf> {
	let search_path = env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/usr/local/bin"));
	for path in env::split_paths(&search_path) {
		let candidate = path.join(prog.as_ref());
		if candidate.exists() {
			if let Ok(true) = access(&candidate) { return Some(candidate) }
		}
	}
	None
}

// Store the paths for compression utilities
lazy_static! {
	pub static ref GZIP_PATH: Option<PathBuf> = find_exec_path("gzip");
	pub static ref PIGZ_PATH: Option<PathBuf> = find_exec_path("pigz");
	pub static ref XZ_PATH: Option<PathBuf> = find_exec_path("xz");
	pub static ref BZIP2_PATH: Option<PathBuf> = find_exec_path("bzip2");
	pub static ref ZSTD_PATH: Option<PathBuf> = find_exec_path("zstd");
	pub static ref LZ4_PATH: Option<PathBuf> = find_exec_path("lz4");
	pub static ref LZMA_PATH: Option<PathBuf> = find_exec_path("lzma");
}

#[derive(Debug)]
pub enum CompressType {
	GZIP,
	COMPRESS,
	BZIP2,
	XZ,
	ZSTD,
	LZ4,
	LZMA,
	UNCOMPRESSED,
}

// Get stored path if present, otherwise returns error
fn get_path<'a>(x: Option<&'a PathBuf>, error_str: &'static str) -> Result<&'a PathBuf> {
	x.ok_or_else(|| Error::new(ErrorKind::Other, format!("Can not find {} executable to uncompress file", error_str)))
}

// Get part to compression utility for a particular compression type
impl CompressType {
	pub fn get_exec_path(&self) -> Result<&PathBuf> {
		match self {
			CompressType::GZIP | CompressType::COMPRESS => get_path(PIGZ_PATH.as_ref().or_else(|| GZIP_PATH.as_ref()).or_else(|| ZSTD_PATH.as_ref()), "pigz, gzip or zstd"),
			CompressType::BZIP2 => get_path(BZIP2_PATH.as_ref(), "bzip2"),
			CompressType::XZ => get_path(XZ_PATH.as_ref().or_else(|| ZSTD_PATH.as_ref()), "xz or zstd"),
			CompressType::LZ4 => get_path(LZ4_PATH.as_ref().or_else(|| ZSTD_PATH.as_ref()), "lz4 or zstd"),
			CompressType::LZMA => get_path(LZMA_PATH.as_ref().or_else(|| ZSTD_PATH.as_ref()), "lzma or zstd"),
			CompressType::ZSTD => get_path(ZSTD_PATH.as_ref(), "zstd"),
			CompressType::UNCOMPRESSED => Err(Error::new(ErrorKind::Other, "Can not get filter path for uncompressed file".to_string())),
		}
	}	
}

// Open a read filter (execute program and return a pipe from the stdout)
pub fn open_read_filter<P: AsRef<Path>, I, S>(prog: P, args: I) -> Result<ChildStdout> 
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>, 
{
	let path: &Path = prog.as_ref();
	match Command::new(path).args(args).stdout(Stdio::piped()).spawn() {
		Ok(proc) => Ok(proc.stdout.expect("pipe problem")),
		Err(error) => Err(Error::new(ErrorKind::Other, format!("Error executing pipe command '{}': {}", path.display(), error))),
	}
}

// Attach a read filter to an existing pipe
pub fn new_read_filter_from_pipe<P: AsRef<Path>>(prog: P, pipe: Stdio) -> Result<ChildStdout> {
	let path: &Path = prog.as_ref();
    match Command::new(path).arg("-d")
        .stdin(pipe)
        .stdout(Stdio::piped())
        .spawn() {
            Ok(proc) => Ok(proc.stdout.expect("pipe problem")),
            Err(error) => Err(Error::new(ErrorKind::Other, format!("Error executing pipe command '{} -d': {}", path.display(), error))),
        }
}

// Open a write filter
pub fn open_write_filter<P: AsRef<Path>, I, S>(file: std::fs::File, prog: P, args: I) -> Result<ChildStdin> 
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>, 
{
	let path: &Path = prog.as_ref();
	match Command::new(path).args(args).stdout(file).stdin(Stdio::piped()).spawn() {
		Ok(proc) => Ok(proc.stdin.expect("pipe problem")),
		Err(error) => Err(Error::new(ErrorKind::Other, format!("Error exectuing pipe command '{}': {}", path.display(), error))),
	}
}

// Try to open a file for reading
fn test_open_file(path: &Path) -> Result<std::fs::File> {
    match File::open(path) {
        Ok(handle) => Ok(handle),
        Err(error) => Err(Error::new(ErrorKind::Other, format!("Error opening {} for input: {}", path.display(), error))),
    }
}

// Guess compression type by looking at first six bytes
fn get_compress_type(path: &Path) -> Result<CompressType> {
    let mut f = test_open_file(path)?;
    let mut buf = [0; 6];
    let n = match f.read(&mut buf) {
        Ok(num) => num,
        Err(error) => return Err(Error::new(ErrorKind::Other, format!("Error reading from {}: {}", path.display(), error))),
    };
    
// Check first bytes of file for the magix numbers indicating different compressed file types
    let mut ctype = CompressType::UNCOMPRESSED;    
    if n == 6 {
        if buf[0] == 0x1f {
            if buf[1] == 0x9d {
                ctype = CompressType::COMPRESS;
            } else if buf[1] == 0x8b && buf[2] == 0x08 {
                ctype = CompressType::GZIP;
            }
        } else if buf[0] == b'B' && buf[1] == b'Z' && buf[2] == b'h' && buf[3] >= b'0' && buf[3] <= b'9' {
            ctype = CompressType::BZIP2;
        } else if buf[0] == 0xfd && buf[1] == b'7' && buf[2] == b'z' && buf[3] == b'X' && buf[4] == b'Z' && buf[5] == 0x00 {
            ctype = CompressType::XZ;
        } else if buf[0] == 0x28 && buf[1] == 0xB5 && buf[2] == 0x2F && buf[3] == 0xFD {
			ctype = CompressType::ZSTD;
        } else if buf[0] == 0x04 && buf[1] == 0x22 && buf[2] == 0x4D && buf[3] == 0x18 {
			ctype = CompressType::LZ4;
        } else if buf[0] == 0x5D && buf[1] == 0x0 && buf[2] == 0x0 {
			ctype = CompressType::LZMA;
		} 
    }
    Ok(ctype)
}

pub enum ReadType {
	Pipe(ChildStdout),
	File(File),	
}

// Create a reader either directly from a file or via a filter if compressed
pub fn open_reader<P: AsRef<Path>>(name: P) -> Result<ReadType> {
	let ctype = get_compress_type(name.as_ref())?;
	let f = test_open_file(name.as_ref())?;
	match ctype {
		CompressType::UNCOMPRESSED => Ok(ReadType::File(f)),
		_ => new_read_filter_from_pipe(ctype.get_exec_path()?, Stdio::from(f)).map(ReadType::Pipe),
	}
}

// Returns a BufReader for file "name"
pub fn open_bufreader<P: AsRef<Path>>(name: P) -> Result<Box<dyn BufRead>> {
	match open_reader(name)? {
		ReadType::File(file) => Ok(Box::new(BufReader::new(file))),
		ReadType::Pipe(pipe) => Ok(Box::new(BufReader::new(pipe))),
	}
}

// Return a BufReader either for file "name" or stdin
pub fn get_reader<P: AsRef<Path>>(name: Option<P>) -> Result<Box<dyn BufRead>> {
    match name {
        Some(file) => open_bufreader(file),
        None => Ok(Box::new(BufReader::new(stdin()))),
    }
}

// Create a BufWriter to file "path"
pub fn open_bufwriter<P: AsRef<Path>>(path: P) -> Result<Box<dyn Write>> {
	let file = File::create(path)?;
	Ok(Box::new(BufWriter::new(file)))
}

// Create a BufWriter either to file "path" or stdout
pub fn get_writer<P: AsRef<Path>>(name: Option<P>) -> Result<Box<dyn Write>> {
	match name {
		Some(file) => open_bufwriter(file),
		None => Ok(Box::new(BufWriter::new(stdout())))
	}
}

// Create a BufWriter passing the output through a filter, storing the output from the filter in "path"
pub fn open_pipe_writer<P: AsRef<Path>, Q: AsRef<Path>, I, S>(path: P, prog: Q, args: I) -> Result<Box<dyn Write>> 
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>, 
{
	let file = File::create(path)?;
	Ok(Box::new(BufWriter::new(open_write_filter(file, prog, args)?)))	
}
