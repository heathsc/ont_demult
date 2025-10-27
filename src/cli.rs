use anyhow::Context;
use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command, crate_version, value_parser};

use super::{DEFAULT_PREFIX, Param, ParamBuilder, log_level::LogLevel, strategy::Strategy};
use crate::cut_site::read_cut_file;
use crate::log_level::init_log;

fn command_line() -> ArgMatches {
    Command::new("ont_demult").version(crate_version!()).author("Simon Heath")
       .about("Takes a paf file (from minimap2) and a list of cut sites and will categorize reads based on the starting points relative to sut sites")
       .arg(
           Arg::new("loglevel")
               .short('l')
               .long("loglevel")
               .value_name("LOGLEVEL")
               .value_parser(value_parser!(LogLevel))
               .ignore_case(true)
               .default_value("info")
               .help("Set log level"),
       )
       .next_help_heading("Selection")
       .arg(
           Arg::new("select")
              .short('S').long("select")
              .value_name("STRATEGY")
              .value_parser(value_parser!(Strategy))
              .ignore_case(true).default_value("start")
              .help("Read selection strategy"),
       )
       .arg(
           Arg::new("mapq_threshold")
              .short('q').long("mapq-threshold")
              .value_name("MAPQ").default_value("10")
              .value_parser(value_parser!(u8))
              .help("MAPQ quality threshold"),
       )
       .arg(
           Arg::new("max_distance")
              .short('m').long("max-distance")
              .value_name("INT").default_value("50")
              .value_parser(value_parser!(usize))
              .help("Maximum distance allowed between cut-site and starting read position"),
       )
       .arg(
           Arg::new("max_unmatched")
              .short('u').long("max-unmatched")
              .value_name("INT").default_value("200")
              .value_parser(value_parser!(usize))
              .help("Maximum number of bases in a read that can be unmatched"),
       )
       .arg(
           Arg::new("margin")
              .short('x').long("margin")
              .value_name("INT").default_value("0")
              .value_parser(value_parser!(usize))
              .help("Extra distance at start of reads on 'other side' of cut site"),
       )
       .next_help_heading("Input/Output")
       .arg(
           Arg::new("cut_file")
              .short('f').long("cut-file")
              .value_name("FILE")
              .value_parser(value_parser!(PathBuf))
              .help("File with details of cut sites"),
       )
       .arg(
           Arg::new("fastq")
              .short('F').long("fastq")
              .value_name("FILE")
              .value_parser(value_parser!(String))
              .help("Input FASTQ file for demultiplexing"),
       )
       .arg(
           Arg::new("matched_only")
              .short('M').long("matched-only")
              .action(ArgAction::SetTrue)
              .help("Only output matched FASTQ records [default: Output all FASTQ records]"),
       )
       .arg(
           Arg::new("prefix")
              .short('p').long("prefix")
              .value_name("PREFIX")
              .value_parser(value_parser!(String))
              .default_value(DEFAULT_PREFIX)
              .help("Prefix for file names"),
       )
       .arg(
           Arg::new("compress")
              .short('z').long("compress")
              .action(ArgAction::SetTrue)
              .help("Compress output files with gzip"),
       )
       .arg(
           Arg::new("paf_file")
              .value_name("Input PAF file")
                .value_parser(value_parser!(String))
              .help("Input PAF file [default: <stdin>]"),
       )
       .get_matches()
}

pub fn process_cli() -> anyhow::Result<Param> {
    //    let yaml = load_yaml!("cli/cli.yml");
    //    let app = App::from_yaml(yaml).version(crate_version!());

    let m = command_line();

    // Setup logging
    init_log(&m);

    // Build param structure from options
    let mut pb = ParamBuilder::new();

    if let Some(file) = m.get_one::<String>("fastq") {
        pb.fastq_file(file);
    }

    if let Some(file) = m.get_one::<String>("paf_file") {
        pb.paf_file(file);
    }

    // Process cut file if present
    if let Some(file) = m.get_one::<PathBuf>("cut_file") {
        pb.cut_sites(read_cut_file(file).with_context(|| "Error reading cut sites from file")?);
    }

    pb.prefix(m.get_one::<String>("prefix").unwrap())
        .compress(m.get_flag("compress"))
        .matched_only(m.get_flag("matched_only"))
        .mapq_thresh(
            *m.get_one::<u8>("mapq_threshold")
                .ok_or(anyhow!("Missing argument to mapq-threshold option"))?,
        )
        .max_distance(
            *m.get_one("max_distance")
                .ok_or(anyhow!("Missing argument to map-distance option"))?,
        )
        .max_unmatched(
            *m.get_one("max_unmatched")
                .ok_or(anyhow!("Missing argument to max-unmatched option"))?,
        )
        .margin(
            *m.get_one("margin")
                .ok_or(anyhow!("Missing argument to margin option"))?,
        )
        .select(
            *m.get_one("select")
                .ok_or(anyhow!("Invalid argument to select option"))?,
        );

    Ok(pb.build())
}
