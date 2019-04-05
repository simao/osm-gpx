extern crate quick_xml;
extern crate geo_types;
extern crate gpx;

use std::fs::File;
use quick_xml::events::attributes::Attributes;
use std::str;

use gpx::write;
use gpx::Gpx;
use gpx::GpxVersion;
use gpx::Waypoint;
use geo_types::Point;

use quick_xml::Reader;
use quick_xml::events::Event;

fn has_attribute(attrs: &Attributes, name: &str, value: &str) -> bool {
    for att in attrs.clone() {
        let att = att.unwrap();
        if str::from_utf8(att.key).unwrap() == name && str::from_utf8(&att.value).unwrap() == value {
            return true
        }
    };

    false
}

fn extract_attribute(attrs: &Attributes, name: &str) -> Option<String> {
    for att in attrs.clone() {
        let att = att.unwrap();
        if str::from_utf8(att.key).unwrap() == name {
            return Some(String::from_utf8(att.value.to_vec()).unwrap())
        }
    };

    None
}

fn extract_coordinates(attrs: &Attributes) -> Option<(f64, f64)> {
    let mut lat = None;
    let mut lon = None;

    for att in attrs.clone() {
        let att = att.unwrap();

        if str::from_utf8(att.key).unwrap() == "lat" {
            lat = Some(String::from_utf8(att.value.to_vec()).unwrap())
        } else if str::from_utf8(att.key).unwrap() == "lon" {
            lon = Some(String::from_utf8(att.value.to_vec()).unwrap())
        }
    }

    lat.and_then(|a| lon.map(|b| (a.parse::<f64>().unwrap(), b.parse::<f64>().unwrap())))
}

fn write_gpx_data(data: Gpx) -> std::io::Result<()> {
    let buffer = File::create("foo.xml")?;
    write(&data, buffer).unwrap();
    Ok(())
}

fn main() {
    let mut reader = Reader::from_file("sachsen-anhalt-latest.osm").unwrap();
    // let mut reader = Reader::from_file("test.xml").unwrap();
    let mut buf = Vec::new();

    let mut is_campsite = false;
    let mut coordinates = None;
    let mut campsite_name = None;

    let mut data : Gpx = Default::default();
    data.version = GpxVersion::Gpx11;
    data.waypoints = vec![];

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) if e.name() == b"node" => {
                coordinates = extract_coordinates(&mut e.attributes());
            }
            Ok(Event::Empty(ref e)) if e.name() == b"tag" => {
                let attributes = e.attributes();

                if has_attribute(&attributes, "k", "tourism") && has_attribute(&attributes, "v", "camp_site") {
                    is_campsite = true;
                } else if has_attribute(& attributes, "k", "name") {
                    campsite_name = extract_attribute(&attributes, "v");
                }
            }
            Ok(Event::End(ref e)) if e.name() == b"node" => {
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
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }

        buf.clear();
    }

    write_gpx_data(data).unwrap()
}
