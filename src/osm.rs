use geo_types::Point;
use gpx::Waypoint;
use osmpbfreader::{OsmId, OsmObj};
use std::collections::BTreeMap;
use geo::algorithm::centroid::Centroid;
use geo::MultiPoint;

// This needs to recurse because some nodes are tagged with the
// expression we are looking for, but they are at the edge of a large
// area, so we need to find the other nodes and calculate the center
// of the area to give an accurate position
pub fn extract_gpx_waypoint_recur(
    objs: &BTreeMap<OsmId, OsmObj>,
    start_at: &OsmObj,
    default_waypoint_name: Option<&str>,
) -> Option<Waypoint> {
    let name = extract_name(start_at).or(default_waypoint_name.map(|s| s.to_owned()));
    let mut deps = extract_osm_obj_deps(start_at);
    let mut result = None;

    while result.is_none() && deps.len() > 0 {
        let obj = deps.pop().unwrap();

        if deps.len() > 1000 {
            panic!(
                "Something went wrong, too many dependencies to search for. Started with {:?}",
                start_at
            );
        }

        match obj {
            OsmId::Node(ref id) => {
                let node = objs.get(&OsmId::from(*id)).and_then(|n| n.node());

                if let Some(n) = node {
                    result = Some(build_waypoint_from_node(name, n));
                    break;
                }
            }
            OsmId::Way(ref id) => {
                let way = objs.get(&OsmId::Way(*id)).unwrap();
                let mut node_ids = extract_osm_obj_deps(way);
                let nodes = node_ids
                    .iter()
                    .flat_map(|n| objs.get(&n).and_then(|nn| nn.node()))
                    .collect();

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

fn extract_name(obj: &osmpbfreader::OsmObj) -> Option<String> {
    obj.tags().get("name").map(|c| c.to_owned())
}

fn build_waypoint_from_node(name: Option<String>, node: &osmpbfreader::objects::Node) -> Waypoint {
    let point = Point::new(node.lon(), node.lat());
    build_waypoint_from_point(name, &point)
}

fn build_waypoint_from_point(name: Option<String>, point: &Point<f64>) -> Waypoint {
    let mut wpt = Waypoint::new(*point);
    wpt.name = name;
    info!("Found node named {:?} at {:?}", wpt.name, point);
    wpt
}

fn calculate_centroid(nodes: &Vec<&osmpbfreader::objects::Node>) -> Option<Point<f64>> {
    let points: Vec<Point<f64>> = nodes.iter().map(|n| Point::new(n.lon(), n.lat())).collect();
    let multi_point: MultiPoint<_> = points.into();
    multi_point.centroid()
}

fn extract_osm_obj_deps(obj: &OsmObj) -> Vec<OsmId> {
    match obj {
        OsmObj::Node(ref _node) => vec![obj.id()],
        OsmObj::Way(ref way) => way.nodes.iter().map(|n| OsmId::from(*n)).collect(),
        OsmObj::Relation(ref relation) => relation.refs.iter().map(|m| m.member).collect(),
    }
}
