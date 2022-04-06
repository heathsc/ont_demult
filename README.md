# ont_demult
Utility to demultiplex ONT reads using mapping to Cas9 cut sites to split the reads

- [Introduction](#Introduction)
- [Installation](#Installation)
- [Usage](#Usage)
    - [Command line options](#Command-line-options)
    - [Cut file](#Cut-file)
    - [Selection strategies](#Selection-strategies)
      - [Start](#Start)
      - [Both](#Both)
      - [Either](#Either)
      - [Xor](#Xor)
    - [Output files](#Output-files) 
      - [Results file](#Results-file)
      - [FASTQ files](#FASTQ-files)
- [Changes](#Changes)

## Introduction

Ont_demult is a demultiplexing utility for sequence reads coming from ONT sequencers,  It is intended
for library preparations that use Cas9 to cut the 
DNA at specific sites allowing DNA from different samples using different Cas9 guides (and so different cut sites)
to be multiplexed in the same flowcell.  The basic operation of ont_demult
is to assign reads to a particular cut site (and therefore to a sample) and, optionally, to
separate a supplied FASTQ file into cut site specific FASTQ files.  

Several strategies are available for demultiplexing with different levels of stringency.  Ont_demult was developed to handle mitochondrial DNA and was designed for circular genomes, but can also be used for
linear although not all of the demultiplexing strategies are applicable to linear genomes.

## Installation

To compile you will need an up-to-date copy of rust.  This can be
installed locally following the instructions [here](https://www.rust-lang.org/learn/get-started).  
Note that if you have rust already installed you should update it
using ``rustup update`` before trying to compile ont_demult.

Clone the ont_demult repository and then from the ont_demult directory
use cargo to compile the application:

    cargo build --release
	 
After successful the executable will be found in target/release/.  It
should be copied somewhere where it can be found by the shell.

Once installed, basic help can be found by invoking ont_demult with
the -h flag.

## Usage

ont_demult works with a PAF alignment file (required), a file with a
description of the CRISPR cut sites (optional) and a FASTQ file
corresponding to the PAF file (optional).  If all three files are
specified then ont_demult will demultiplex the FASTQ file using the
cut site information to separate the reads into Unmapped, Low Mapq,
Unmatched and Matched categories.

### Command line options
Ont_demult has many command line options for controlling the operation of the process.

| Short | Long           | Description                                                          | Default    |
|-------|----------------|----------------------------------------------------------------------|------------|
| s     | select         | Read selection strategy (start, both, either ,xor)                   | start      |
| q     | mapq-threshold | MAPQ threshold                                                       | 10         |
| m     | max-distance   | Maximum distance allowed between cut-site and starting read position | 100        |
| u     | max-unmatched  | Maximum number of bases in a read that can be unmatched              | 200        |
| x     | margin         | Extra distance at start of reads on 'other side' of cut site         | 10         |
|||||
| f     | cut-file       | File with details of cut sites                                       |            |
| F     | fastq          | Input FASTQ file for demultiplexing                                  |            |
| p     | prefix         | Prefix string for output files                                       | ont_demult |
| M     | matched-only   | Only output FASTQ records that are matched to a cut site             |            |
| z     | compress       | Compress output files with GZIP                                      |            |

### Cut file

The cut file provides the details of the cut sites and the association between samples nad cut sites.
The file is a tab separated text file with no header line with the following format

| Chromosome | position | cut site name | sample | circular genome |
|------------|----------|---------------|--------|-----------------|

The last column is an indicator of whether the genome is circular: it should be
**true / yes / 1** if the genome is circular and **false / no / 0** if the genome is linear.
The position column is 1 offset, and should be the position just after the cut site, i.e., the expected position 
of the first base of the cut strand.  An example cut file is given below.

```
chrM    1006    mt_1kb  Sample1 true
chrM    9338    mt_9kb  Sample1 true
chrM    3127    mt_3kb  Sample2 true
chrM    11239   mt_11kb Sample2 true
chrM    5142    mt_5kb  Sample3 true
chrM    12767   mt_13kb Sample3 true
chrM    7144    mt_7kb  Sample4 true
chrM    14968   mt_15kb Sample4 true
```

### Selection strategies

The principle task of ont_demult is to attempt to match reads to cut sites.  There are multiple strategies
for how reads are selected and matched to cut sites that can be selected.  The selection mode is chosen
using the `--select` command line option.

Whatever selection mode is chosen, the initial processing of the reads is the same.  All of the 
alignments coming from a read are collected together.  Reads that are longer than the target chromosome length 
are filtered out at this stage.  The longest alignment with a MAPQ score >= the threshold (set using the ``--maxq-threshold`` option)
is identified, and all other alignments on the same strand of the same chromosome with MAPQ >= 0 are selected.
The selected alignments are sorted by their position on the read, and a check is made 
that the different alignments form distinct
(non-overlapping) segments of the read.  Any reads that have overlapping segments, or that have excess bases that are not aligned 
(threshold set using the ``--max-unmatched`` option) are discarded.

From the sorted alignments, the map position of the first and last aligned bases of the read are identified;
these are then used to find matching cut sites for each end.  The matching of cut sites to a read is performed
in a strand dependent fashion, and is affected by two parameters, *max-distance* and *margin* than can
be set using the options ``--max-distance`` and ``--margin``.  For a read on the **plus** strand,
the start of the read matches a cut site at *x* if the start position is between 
*x* - *margin* and *x* + *max-distance*.  The end of the read matches a cut site at *y* 
if the end position is between *y* - *max-distance* and *y* + *margin*.  For a read on the
**minus** strand the checks are reversed: the start position matches if it is 
between *x* - *max-distance* and *x* + *margin* and the end position matches if it is between
*y* - *margin* and *y* + *max-distance*.  

At this stage the two ends are matched to cut sites independently.  How the matching of the ends is taken into
account in determining whether a read is selected or not depends on the chosen selection strategy.  The 
four strategies are described below.

#### Start

This is the default strategy.  For a read to be selected, the start of the read much 
match a cut site.  If the end of the read also matches a cut site, it should match *the same* cut site
as the start of the read.

#### Both

This is the most stringent strategy and is only applicable to circular genomes.
For a read to be selected both ends must be matched to the same cut site.  This ensures
that only full length reads are selected. 

#### Either

This is the least stringent strategy, and simply requires that either end matches a cut site.  However,
similar to the start strategy, if both ends of a read match, they must both match the same cut site.

#### Xor

This strategy is mostly for benchmarking rather than normal use.  For a read to be selected either the start should match
a cut site or the end should match, but not both.  This is meant to simulate working with a very degraded sample where no
full length reads exist.

It should be clear from the descriptions above that the set of reads selected by **both** is
a subset of that selected by **start*, which is itself a subset of that selected by **either**.  The
set of reads selected by **xor** is the intersect between the reads selected by **either** and the 
reads *not* selected by **both**.

### Output files

The output files produced by ont_demult are a results file with the results of the matching for each
read found in the input PAF file, and the demultiplexed FASTQ files if an 
input FASTQ file was supplied.

#### Results file

The name of the results file is formed from the output prefix (set with the ``--prefix`` option),
and the ending ``_res.txt`` (with a ``.gz`` suffix if the ``--compress`` option is set).  The results file is a tab separated
text file with a header line.  The columns are as follows:

1. Read ID
2. Match status
3. Cut_site or chromosome if not matched
4. Sample barcode (if matched)
5. Strand (+/-)
6. Location on target of first mapped base
7. Location on target of last mapped base
8. Length of read
9. Number of unmatched bases
10. Proportion of unmatched bases

After the first 10 columns are 0 or more additional pairs of columns with
the start and end mapped positions of splits within the read.

The match status column describes the result of the matching.  A value of *Matched* 
indicates a success full match; all other values indicate that the read was not matched, and 
provide information as to the reason why this was so.
Note that column 3 depends on the match status:
if the read has been matched to a cut site then column 3 shows the matched cut site otherwise it shows the chromosome the read maps to.
An asterix (*) indicates that a value is not available (for example, column 4 shows an asterix for unmatched reads). 
The possible values for match status are given in the table below.  Note that some values will only
be found when using certain selection strategies

| Match status value | Description                                              | Selection strategies |
|--------------------|----------------------------------------------------------|----------------------|
| Matched            | Read matched successfully                                | All                  |
| MatchStart         | Start of read matches a cut site but not the end         | Both                 |
| MatchEnd           | End of read matches a cut site but not the start         | Both, Start          |
| MatchBoth          | Both ends match a cut site                               | Xor                  |
| MisMatch           | The two ends match different cut sites                   | All                  |
| ExcessUnmatched    | Too many bases in the read are not matched to the target | All                  |
| Unmatched          | No match to any cutsite                                  | All                  |
| LowMapQ            | Low MAPQ for read                                        | All                  |
| Unmapped           | Read did not map                                         | All                  |

#### FASTQ files

If an input FASTQ file is provided (with the ``--fastq`` option) then cut site specific output files are created
with the FASTQ records of reads matched to each cut site.  The names of the FASTQ files are formed 
from the output prefix (set with the ``--prefix`` option), the cut site name (from the [cut file](#Cut-file)),
and the ending ``.fastq`` (with a ``.gz`` suffix if the ``--compress`` option is set).

By default, output files are also created for _unmapped_,
_unmatched_ and _low MAPQ_ reads.  If these extra files are **not** required then the ``--matched-only`` option
option will suppress these files and output only the matching reads.  Note that the filenames for these extra
files will have ``unmapped``, ``unmatched`` and ``low_mapq`` in place of the cut site name - do not use any of these
as a cut site name, or it will cause the files to be overwritten!

## Changes

- 0.3.3 Switch to using compress_io from crates.io
- 0.3.2 Fix bug in Xor selection mode where a read only matching the end site would not be selected
- 0.3.1 Correct headers in results file.  Clean up output.
- 0.3.1 Fix compress option which was not being read correctly.
- 0.3.0 Moved to Clap v3.
- 0.3.0 Added documentation in this file.