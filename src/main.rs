extern crate osmpbfreader;
extern crate geo_types;
extern crate gpx;
#[macro_use] extern crate log;
extern crate pretty_env_logger;

use std::fs::File;

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

fn is_campsite(obj: &osmpbfreader::OsmObj) -> bool {
    obj.tags().contains("tourism", "camp_site")
}

fn extract_name(obj: &osmpbfreader::OsmObj) -> Option<String> {
    obj.tags().get("name").map(|c| c.to_owned() )
}

// TODO: Should recurse and get dependent object by id if not a node
fn extract_gpx_waypoint(name: Option<String>, obj: &osmpbfreader::OsmObj) -> Option<Waypoint> {
    match obj {
        osmpbfreader::OsmObj::Node(ref node) => {
            let point = Point::new(node.lat(), node.lon());
            let mut wpt = Waypoint::new(point);
            wpt.name = name;
            info!("Found campsite named {:?} at {:?}", wpt.name, point);
            Some(wpt)
        }
        _ => {
            error!("obj is not a node, cannot extract waypoint: {:?}", obj);
            None
        }
    }
}

fn main() {
    pretty_env_logger::init();

    let filename = "sachsen-latest.osm.pbf";
    let r = std::fs::File::open(&std::path::Path::new(filename)).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);
    let mut counter = 0;

    let mut data : Gpx = Default::default();
    data.version = GpxVersion::Gpx11;
    data.waypoints = vec![];

    let objs = pbf.get_objs_and_deps(is_campsite).unwrap();

    for obj in objs.values() {
        match obj {
            osmpbfreader::OsmObj::Node(_) if is_campsite(obj) => {
                let name = extract_name(&obj);
                let wpt = extract_gpx_waypoint(name, &obj).unwrap();
                data.waypoints.push(wpt);
                counter += 1;
            }
            osmpbfreader::OsmObj::Way(ref way) if is_campsite(obj) => {
                let name = extract_name(&obj);
                let first = way.nodes.first().unwrap();
                let node = objs.get(&osmpbfreader::OsmId::Node(*first)).unwrap();

                if let Some(wpt) = extract_gpx_waypoint(name, node) {
                    data.waypoints.push(wpt);
                    counter += 1;
                }
            }
            osmpbfreader::OsmObj::Relation(ref relation) if is_campsite(obj) => {
                let name = extract_name(&obj);
                let first = relation.refs.first().unwrap().member;
                let node = objs.get(&first).unwrap();

                for r in &relation.refs {
                    warn!("relation ref {:?}", r)
                }

                if let Some(wpt) = extract_gpx_waypoint(name, node) {
                    data.waypoints.push(wpt);
                    counter += 1;
                }
            }
            o =>
                debug!("unmatched obj: {:?}", o)
        }
    }

    info!("finished, found {} matching waypoints, but somehow only have {} waypoints on gpx ", counter, data.waypoints.len());

    write_gpx_data(data).unwrap();
}
