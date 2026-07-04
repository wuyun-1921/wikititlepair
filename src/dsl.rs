use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::Command;

use crate::error::Result;

/// Write entries to a DSL file. Entries are pre-formatted (headword, definition body).
pub fn write_dsl(
    output: &Path,
    name: &str,
    index_lang: &str,
    contents_lang: &str,
    entries: &[(String, String)],
) -> Result<()> {
    let mut file = BufWriter::new(File::create(output)?);

    writeln!(file, "#NAME \"{}\"", name)?;
    writeln!(file, "#INDEX_LANGUAGE \"{}\"", index_lang)?;
    writeln!(file, "#CONTENTS_LANGUAGE \"{}\"", contents_lang)?;
    writeln!(file)?;

    for (headword, body) in entries {
        write!(file, "{}\n\t{}\n", headword, body)?;
    }

    file.flush()?;
    Ok(())
}

/// Compress DSL file with dictzip. Returns false if dictzip unavailable (file kept uncompressed).
pub fn compress_dictzip(path: &Path) -> bool {
    let dz_output = path.with_extension("dsl.dz");
    eprintln!("Compressing with dictzip...");
    match Command::new("dictzip").arg(path.to_str().unwrap()).status() {
        Ok(s) if s.success() => {
            eprintln!("  {} created", dz_output.display());
            true
        }
        _ => {
            eprintln!("  dictzip unavailable or failed (dsl file kept)");
            false
        }
    }
}
