extern crate clap;
extern crate geo_types;
extern crate gpx;
extern crate osmpbfreader;
extern crate regex;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod node;
mod osmgpx;
mod osm;

use node::*;
use osmgpx::*;
use osm::*;

use clap::{App, Arg};
use gpx::Gpx;
use gpx::GpxVersion;

use std::path::Path;


fn main() {
    if let Err(_) = std::env::var("RUST_LOG")  {
        std::env::set_var("RUST_LOG", "osm_gpx=info")
    ;}

    pretty_env_logger::init();

    let matches = App::new("osm-gpx")
        .version("0.1.0")
        .author("Simão Mata <sm@0io.eu>")
        .about("extracts gpx waypoints from OpenStreetMap nodes matching some expression")
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
