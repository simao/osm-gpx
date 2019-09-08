use gpx::write;
use gpx::Gpx;
use std::fs::File;
use std::path::Path;

pub fn write_gpx_data(output: &Path, data: Gpx) -> std::io::Result<()> {
    let buffer = File::create(output)?;
    write(&data, buffer).unwrap();
    Ok(())
}
