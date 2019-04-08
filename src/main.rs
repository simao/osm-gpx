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

use osmpbfreader::{OsmId, OsmObj};
use std::collections::BTreeMap;

use geo::MultiPoint;
use geo::algorithm::centroid::Centroid;

// TODO: Cli arguments
// TODO: Proper Error management

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

fn build_waypoint_from_node(name: Option<String>, node: &osmpbfreader::objects::Node) -> Waypoint {
    let point = Point::new(node.lon(), node.lat());
    build_waypoint_from_point(name, &point)
}

fn build_waypoint_from_point(name: Option<String>, point: &Point<f64>) -> Waypoint {
    let mut wpt = Waypoint::new(*point);
    wpt.name = name;
    info!("Found campsite named {:?} at {:?}", wpt.name, point);
    wpt
}

fn calculate_centroid(nodes: &Vec<&osmpbfreader::objects::Node>) -> Option<Point<f64>> {
    let points: Vec<Point<f64>> = nodes.iter().map(|n| Point::new(n.lon(), n.lat())).collect();
    let multi_point: MultiPoint<_> = points.into();
    multi_point.centroid()
}

fn extract_osm_obj_deps(obj: &OsmObj) -> Vec<OsmId> {
    match obj {
        OsmObj::Node(ref _node) =>
            vec![obj.id()],
        OsmObj::Way(ref way) =>
            way.nodes.iter().map(|n| OsmId::Node(*n)).collect(),
        OsmObj::Relation(ref relation) =>
            relation.refs.iter().map(|m| m.member).collect(),
    }
}

fn extract_gpx_waypoint_recur(objs: &BTreeMap<OsmId, OsmObj>, start_at: &OsmObj) -> Option<Waypoint> {
    let name = extract_name(start_at);
    let mut deps = extract_osm_obj_deps(start_at);
    let mut result = None;

    while result.is_none() && deps.len() > 0 {
        let obj = deps.pop().unwrap();

        if deps.len() > 1000 {
            panic!("Something went wrong, too many dependencies to search for. Started with {:?}", start_at);
        }

        match obj {
            OsmId::Node(ref id) => {
                let node = objs.get(&OsmId::Node(*id)).and_then(|n| n.node());

                if let Some(n) = node {
                    result = Some(build_waypoint_from_node(name, n));
                    break;
                }
            }
            OsmId::Way(ref id) => {
                let way = objs.get(&OsmId::Way(*id)).unwrap();
                let mut node_ids = extract_osm_obj_deps(way);
                let nodes = node_ids.iter().flat_map(|n| objs.get(&n).and_then(|nn| nn.node())).collect();

                if let Some(centroid) = calculate_centroid(&nodes) {
                    result = Some(build_waypoint_from_point(name.clone(), &centroid));
                } else {
                    deps.append(&mut node_ids);
                }
            }
            OsmId::Relation(ref id) => {
                debug!("Dependency is type relation, recursing");
                let relation = objs.get(&OsmId::Relation(*id)).unwrap();
                let mut relations = extract_osm_obj_deps(relation);
                deps.append(&mut relations);
            }
        }
    }

    result
}

fn main() {
    pretty_env_logger::init();

    let filename = "sachsen-latest.osm.pbf";
    let r = std::fs::File::open(&std::path::Path::new(filename)).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);

    let mut data : Gpx = Default::default();
    data.version = GpxVersion::Gpx11;
    data.waypoints = vec![];

    let objs = pbf.get_objs_and_deps(is_campsite).unwrap();

    for o in objs.values() {
        match o {
            obj if is_campsite(&o) => {
                if let Some(wpt) = extract_gpx_waypoint_recur(&objs, &obj) {
                    data.waypoints.push(wpt);
                } else {
                    warn!("Could not recurse to get dependencies for {:?}", obj);
                }
            }
            obj =>
                debug!("unmatched obj: {:?}", obj)
        }
    }

    info!("finished, found {} matching waypoints ", data.waypoints.len());

    write_gpx_data(data).unwrap();
}
