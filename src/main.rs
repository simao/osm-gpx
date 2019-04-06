extern crate osmpbfreader;
extern crate geo_types;
extern crate gpx;

use std::fs::File;
use std::str;

use gpx::write;
use gpx::Gpx;
use gpx::GpxVersion;
use gpx::Waypoint;
use geo_types::Point;

fn write_gpx_data(data: Gpx) -> std::io::Result<()> {
    let buffer = File::create("foo.xml")?;
    write(&data, buffer).unwrap();
    Ok(())
}

fn main() {
    let filename = "sachsen-latest.osm.pbf";
    let r = std::fs::File::open(&std::path::Path::new(filename)).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);

    let mut data : Gpx = Default::default();
    data.version = GpxVersion::Gpx11;
    data.waypoints = vec![];

    for obj in pbf.par_iter().map(Result::unwrap) {
        if obj.tags().contains("tourism", "camp_site") {
            let maybe_point = obj.node().map(|c| Point::new(c.lat(), c.lon()));

            if let Some(point) = maybe_point {
                let mut wpt = Waypoint::new(point);
                wpt.name = obj.tags().get("name").map(|c| c.to_owned() );
                println!("Found campsite named {:?} at {:?}", wpt.name, point);
                data.waypoints.push(wpt);
            } else {
                println!("Could not get coordinates for {:?}", obj);
            }
        }
    }

    write_gpx_data(data).unwrap()
}
