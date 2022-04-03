use anyhow::Context;

use clap::{Command, Arg, ArgMatches, crate_version};

use super::*;
use crate::cut_site::read_cut_file;
use crate::log_level::init_log;

fn command_line() -> ArgMatches {
    Command::new("ont_demult").version(crate_version!()).author("Simon Heath")
       .about("Takes a paf file (from minimap2) and a list of cut sites and will categorize reads based on the starting points relative to sut sites")
       .arg(
           Arg::new("loglevel")
              .short('l').long("loglevel")
              .takes_value(true).value_name("LOGLEVEL")
              .possible_values(&["none", "error", "warn", "info", "debug", "trace"])
              .ignore_case(true).default_value("info")
              .help("Set log level")
       )
       .next_help_heading("Selection")
       .arg(
           Arg::new("select")
              .short('S').long("select")
              .takes_value(true).value_name("STRATEGY")
              .possible_values(["start", "both", "either", "xor"])
              .ignore_case(true).default_value("start")
              .help("Read selection strategy"),
       )
       .arg(
           Arg::new("mapq_threshold")
              .short('q').long("mapq-threshold")
              .takes_value(true).value_name("MAPQ").default_value("10")
              .help("MAPQ quality threshold"),
       )
       .arg(
           Arg::new("max_distance")
              .short('m').long("max-distance")
              .takes_value(true).value_name("INT").default_value("100")
              .help("Maximum distance allowed between cut-site and starting read position"),
       )
       .arg(
           Arg::new("max_unmatched")
              .short('u').long("max-unmatched")
              .takes_value(true).value_name("INT").default_value("200")
              .help("Maximum number of bases in a read that can be unmatched"),
       )
       .arg(
           Arg::new("margin")
              .short('x').long("margin")
              .takes_value(true).value_name("INT").default_value("10")
              .help("Extra distance at start of reads on 'other side' of cut site"),
       )
       .next_help_heading("Input/Output")
       .arg(
           Arg::new("cut_file")
              .short('f').long("cut-file")
              .takes_value(true).value_name("FILE")
              .help("File with details of cut sites"),
       )
       .arg(
           Arg::new("fastq")
              .short('F').long("fastq")
              .takes_value(true).value_name("FILE")
              .help("Input FASTQ file for demultiplexing"),
       )
       .arg(
           Arg::new("matched_only")
              .short('M').long("matched-only")
              .help("Only output matched FASTQ records [default: Output all FASTQ records]"),
       )
       .arg(
           Arg::new("prefix")
              .short('p').long("prefix")
              .takes_value(true).value_name("PREFIX")
              .default_value(DEFAULT_PREFIX)
              .help("Prefix for file names"),
       )
       .arg(
           Arg::new("compress")
              .short('z').long("compress")
              .help("Compress output files with gzip"),
       )
       .arg(
           Arg::new("paf_file")
              .takes_value(true).value_name("Input PAF file")
              .help("Input PAF file [default: <stdin>]"),
       )
       .get_matches()
}

pub fn process_cli() -> anyhow::Result<Param> {
//    let yaml = load_yaml!("cli/cli.yml");
//    let app = App::from_yaml(yaml).version(crate_version!());

    let m = command_line();

    // Setup logging
    let _ = init_log(&m);

    // Build param structure from options
    let mut pb = ParamBuilder::new();

    if let Some(file) =  m.value_of("fastq") {
        pb.fastq_file(file);
    }

    if let Some(file) =  m.value_of("paf_file") {
        pb.paf_file(file);
    }

    // Process cut file if present
    if let Some(file) = m.value_of("cut_file") {
        pb.cut_sites(read_cut_file(file).with_context(|| "Error reading cut sites from file")?);
    }

    pb.prefix(m.value_of("prefix").unwrap())
       .compress(m.is_present("compress)"))
       .matched_only(m.is_present("matched_only"))
       .mapq_thresh(m.value_of_t("mapq_threshold").with_context(|| "Invalid argument to mapq_threshold option")?)
       .max_distance(m.value_of_t("max_distance").with_context(|| "Invalid argument to map_distance option")?)
       .max_unmatched(m.value_of_t("max_unmatched").with_context(|| "Invalid argument to max_unmatched option")?)
       .margin(m.value_of_t("margin").with_context(|| "Invalid argument to margin option")?)
       .select(m.value_of_t("select").with_context(|| "Invalid argument to select option")?)
       ;

   Ok(pb.build())
}
