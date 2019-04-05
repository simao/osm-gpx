extern crate xml;
extern crate geo_types;
extern crate gpx;

use std::fs::File;
use std::io::BufReader;

use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;

fn has_attribute(attrs: &Vec<OwnedAttribute>, name: &str, value: &str) -> bool {
    for att in attrs {
        if att.name.local_name == name && att.value == value {
            return true
        }
    };

    false
}

fn extract_attribute(attrs: &Vec<OwnedAttribute>, name: &str) -> Option<String> {
    for att in attrs {
        if att.name.local_name == name {
            return Some(att.value.clone())
        }
    };

    None
}

fn extract_coordinates(attrs: &Vec<OwnedAttribute>) -> Option<(f64, f64)> {
    let mut lat = None;
    let mut lon = None;

    for att in attrs {
        if att.name.local_name == "lat" {
            lat = Some(att.value.clone())
        } else if att.name.local_name == "lon" {
            lon = Some(att.value.clone())
        }
    }

    lat.and_then(|a| lon.map(|b| (a.parse::<f64>().unwrap(), b.parse::<f64>().unwrap())))
}

use gpx::write;
use gpx::Gpx;
use gpx::GpxVersion;
use gpx::Waypoint;
use geo_types::Point;

fn write_gpx_data(data: Gpx) -> std::io::Result<()> {
    let mut buffer = File::create("foo.xml")?;
    write(&data, buffer).unwrap();
    Ok(())
}

fn main() {
    let file = File::open("sachsen-anhalt-latest.osm").unwrap();
    let file = BufReader::new(file);

    let parser = EventReader::new(file);

    let mut is_campsite = false;
    let mut coordinates = None;
    let mut campsite_name = None;

    let mut data : Gpx = Default::default();
    data.version = GpxVersion::Gpx11;
    data.waypoints = vec![];

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { ref name, ref attributes, .. }) if name.local_name == "node"=> {
                coordinates = extract_coordinates(attributes);
            },
            Ok(XmlEvent::StartElement { ref name, ref attributes, .. }) if name.local_name == "tag"=> {
                if has_attribute(&attributes, "k", "tourism") && has_attribute(&attributes, "v", "camp_site") {
                    is_campsite = true;
                } else if has_attribute(&attributes, "k", "name") {
                    campsite_name = extract_attribute(&attributes, "v");
                }
            }
            Ok(XmlEvent::EndElement { ref name, .. }) if name.local_name == "node"=> {
                if is_campsite {
                    println!("Found campsite named {:?} at {:?}", campsite_name, coordinates);
                    let point = coordinates.map(|c| Point::new(c.1, c.0) ).unwrap();
                    let mut wpt = Waypoint::new(point);
                    wpt.name = campsite_name;
                    data.waypoints.push(wpt);
                }

                coordinates = None;
                is_campsite = false;
                campsite_name = None;
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    write_gpx_data(data).unwrap()
}
