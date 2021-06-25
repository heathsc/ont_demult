# ont_demult
Utility to demultiplex ONT reads using mapping to CRISPR cut sites to split the reads

To compile you will need an up to date copy of rust.  This can be
installed locally following the instructions [here](https://www.rust-lang.org/learn/get-started)).  
Note that if you have rust already installed you should update it
using ``rustup update`` before trying to compile ont_demult.

Clone the ont_demult repository and then from the ont_demult directory
use cargo to compile the application:

    cargo build --release
	 
After successful the executable will be found in target/release/.  It
should be copied somewhere where it can be found by the shell.

Once installed, basic help can be found by invoking ont_demult with
the -h flag.

---------------
Basic operation
---------------

ont_demult works with a PAF alignment file (required), a file with a
description of the CRISPR cut sites (optional) and a FASTQ file
corresponding to the PAF file (optional).  If all three files are
specified then ont_demult will demultiplex the FASTQ file using the
cut site information to separate the reads into Unmapped, Low Mapq,
Unmatched and Matched categories.


   
