# OSM to GPX Node extractor

It's usually very difficult to display POI (Points of Interest) in a mapping application in a way that is easy to see the POIs you are interested when you zoom out. For example, you might want to see all the places where there might be drinking water available in a large area, but most mapping applications do not support this. Most mapping applications support the GPX format to display way points, so if we can create gpx files containing just the waypoints we are intrested in, this would solve our problem.

This tool extracts OSM POIs from OSM data files, available at [Geofabrik](http://download.geofabrik.de/), and builds GPX files based on tag expressions.

## Examples

Extract all places in Berlin/Brandenburg what have drinking water:

    cargo run -- --osm-file brandenburg-latest.osm.pbf --name Water --output brandenburg-water.gpx --exp "amenity=drinking_water"
    
In Europe you usually can find drinking water in cemeteries, so you could run the following to find all cemeteries in Germany:

    cargo run -- --osm-file germany-latest.osm.pbf --name Cemetery --output germany-cemetery.gpx --exp "landuse=cemetery"
    
Other expressions are also supported, check `--help`.

You can then use these files to display POIs in most mapping applications easily.
