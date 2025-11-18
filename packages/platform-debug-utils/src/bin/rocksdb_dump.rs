use clap::Parser;
use rocksdb::{IteratorMode, Options, DB};
use std::{
    error::Error,
    fs::File,
    io::{self, BufWriter, Write},
    path::PathBuf,
};

/// Export RocksDB contents to a diff-friendly text format.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to the RocksDB directory.
    #[arg(long)]
    db_path: PathBuf,

    /// Output file path. If omitted, the export is printed to STDOUT.
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut options = Options::default();
    options.create_if_missing(false);
    options.create_missing_column_families(false);

    let mut cf_names = DB::list_cf(&options, &args.db_path)?;
    if cf_names.is_empty() {
        return Err("database does not contain any column families".into());
    }

    cf_names.sort();

    let db = DB::open_cf_for_read_only(&options, &args.db_path, cf_names.iter(), false)?;

    let writer: Box<dyn Write> = match args.output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };

    let mut writer = BufWriter::new(writer);

    writeln!(
        writer,
        "# RocksDB export from {}",
        args.db_path.display()
    )?;

    for (cf_index, cf_name) in cf_names.iter().enumerate() {
        let cf_handle = db
            .cf_handle(cf_name)
            .ok_or_else(|| format!("column family {cf_name} not found"))?;

        if cf_index > 0 {
            writeln!(writer)?;
        }

        writeln!(writer, "# column_family={cf_name}")?;

        let iter = db.iterator_cf(cf_handle, IteratorMode::Start);

        for item in iter {
            let (key, value) = item?;
            let key_hex = hex::encode(&key);
            let value_hex = hex::encode(&value);

            writeln!(
                writer,
                "cf={cf_name}\tkey=0x{key_hex}\tvalue=0x{value_hex}\tkey_len={}\tvalue_len={}",
                key.len(),
                value.len()
            )?;
        }
    }

    writer.flush()?;
    Ok(())
}
