extern crate clap;
extern crate geo_types;
extern crate gpx;
extern crate osmpbfreader;
extern crate regex;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use clap::{App, Arg};
use geo_types::Point;
use gpx::write;
use gpx::Gpx;
use gpx::GpxVersion;
use gpx::Waypoint;
use regex::Regex;
use std::fs::File;

use osmpbfreader::{OsmId, OsmObj};
use std::collections::BTreeMap;

use geo::algorithm::centroid::Centroid;
use geo::MultiPoint;
use std::path::Path;

// TODO: Proper Error management

fn write_gpx_data(output: &Path, data: Gpx) -> std::io::Result<()> {
    let buffer = File::create(output)?;
    write(&data, buffer).unwrap();
    Ok(())
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

fn extract_gpx_waypoint_recur(
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

#[derive(Debug)]
enum Operator {
    Equals,
    Includes,
}

#[derive(Debug)]
struct NodeExpression {
    tag_name: String,
    tag_value: String,
    op: Operator,
}

impl NodeExpression {
    fn parse(expression: String) -> Result<NodeExpression, String> {
        let re =
            Regex::new(r"(?P<name>\w+)(?P<op>[=~])(?P<value>\w+)").map_err(|e| e.to_string())?;
        let err = format!("Could not compile expression from {}", expression);
        let caps = re.captures(&expression).ok_or(err)?;

        let op = if caps.name("op").unwrap().as_str() == "=" {
            Operator::Equals
        } else {
            Operator::Includes
        };

        Ok(NodeExpression {
            tag_name: caps.name("name").unwrap().as_str().into(),
            tag_value: caps.name("value").unwrap().as_str().into(),
            op: op,
        })
    }

    fn matcher(&self) -> impl Fn(&OsmObj) -> bool + '_ {
        move |obj: &OsmObj| match self.op {
            Operator::Equals => obj.tags().contains(&self.tag_name, &self.tag_value),
            Operator::Includes => obj.tags().get(&self.tag_name).map_or(false, |v| {
                v.to_lowercase().contains(&self.tag_value.to_lowercase())
            }),
        }
    }
}

fn main() {
    pretty_env_logger::init();

    let matches = App::new("osm-gpx")
        .version("0.1.0")
        .author("Simão Mata <sm@0io.eu>")
        .about("extracts gpx waypoints for osm nodes matching given tags")
        .arg(
            Arg::with_name("osm-file")
                .short("i")
                .long("osm-file")
                .required(true)
                .value_name("FILE")
                .help("Sets path for osm FILE"),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .required(true)
                .value_name("OUTPUT")
                .help("Sets path for output GPX file"),
        )
        .arg(
            Arg::with_name("expression")
                .short("e")
                .long("exp")
                .required(true)
                .value_name("EXPRESSION")
                .help("Sets expression to search for in the form tag-name=tag-contains"),
        )
        .arg(
            Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("NAME")
                .help("Use NAME as the name of the each waypoint in the gpx if a name is not defined in the data"),
        )        
        .get_matches();

    let filename = matches.value_of("osm-file").unwrap();
    let output = Path::new(matches.value_of("output").unwrap());
    let r = std::fs::File::open(&Path::new(filename)).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);

    let mut data: Gpx = Default::default();
    data.version = GpxVersion::Gpx11;
    data.waypoints = vec![];

    let exp = matches.value_of("expression").unwrap();
    let node_expression = NodeExpression::parse(exp.into()).unwrap();
    let node_matcher = node_expression.matcher();
    let default_waypoint_name = matches.value_of("name");

    let objs = pbf
        .get_objs_and_deps(node_expression.matcher())
        .expect(&format!(
            "Could not open file {}, is the file in osm pbf format?",
            &filename
        ));

    for o in objs.values() {
        match o {
            obj if node_matcher(&o) => {
                if let Some(wpt) = extract_gpx_waypoint_recur(&objs, &obj, default_waypoint_name) {
                    data.waypoints.push(wpt);
                } else {
                    warn!("Could not recurse to get dependencies for {:?}", obj);
                }
            }
            obj => debug!("unmatched obj: {:?}", obj),
        }
    }

    info!(
        "finished, found {} matching waypoints ",
        data.waypoints.len()
    );

    write_gpx_data(output, data).unwrap();
}
