mod latlong_ratios;

use latlong_ratios::get_lat_ratio;
use latlong_ratios::get_long_ratio;

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::env;
use std::fs;
use std::process;

/// Determines if a polygon follows the right-hand rule (counterclockwise)
fn is_clockwise(coordinates: &Vec<(f64, f64)>) -> bool {
    let mut sum = 0.0;
    let n = coordinates.len();
    for i in 0..n {
        let (x1, y1) = coordinates[i];
        let (x2, y2) = coordinates[(i + 1) % n]; // Wrap around to first point
        sum += (x2 - x1) * (y2 + y1);
    }
    sum > 0.0 // Clockwise if sum is positive
}

/// Writes the polygon coordinates to a GeoJSON file
fn write_geojson(filename: &str, coordinates: &Vec<(f64, f64)>) -> io::Result<()> {
    let output_file_path = format!("{}.geojson", filename);
    let mut output_file = File::create(output_file_path.clone())?;

    // Ensure the polygon follows the right-hand rule
    let mut corrected_coordinates = coordinates.clone();
    if is_clockwise(&corrected_coordinates) {
        corrected_coordinates.reverse(); // Reverse to make counterclockwise
    }

    writeln!(output_file, "{{")?;
    writeln!(output_file, "  \"type\": \"FeatureCollection\",")?;
    writeln!(output_file, "  \"features\": [")?;
    writeln!(output_file, "    {{")?;
    writeln!(output_file, "      \"type\": \"Feature\",")?;
    writeln!(output_file, "      \"geometry\": {{")?;
    writeln!(output_file, "        \"type\": \"Polygon\",")?;
    writeln!(output_file, "        \"coordinates\": [")?;
    writeln!(output_file, "          [")?;

    // Iterate through coordinates and write them correctly
    for (i, (long, lat)) in corrected_coordinates.iter().enumerate() {
        if i < corrected_coordinates.len() {
            writeln!(output_file, "          [{}, {}],", long, lat)?;  // Ensure comma after each coordinate
        } else {
            writeln!(output_file, "          [{}, {}]", long, lat)?;   // No comma for the last original coordinate
        }
    }

    // ✅ Ensure the polygon closes correctly
    if let Some(first) = corrected_coordinates.first() {
        writeln!(output_file, "          [{}, {}]", first.0, first.1)?; // Ensures closure without error
    }

    writeln!(output_file, "          ]")?;
    writeln!(output_file, "        ]")?;
    writeln!(output_file, "      }},")?;
    writeln!(output_file, "      \"properties\": {{}}")?;
    writeln!(output_file, "    }}")?;
    writeln!(output_file, "  ]")?;
    writeln!(output_file, "}}")?;

    println!("GeoJSON file generated successfully: {}", output_file_path);
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    let base_filename = Path::new(filename)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string(); // Extract filename without extension

    let data = fs::read_to_string(filename).expect("Unable to read file");

    println!("What units are used in your data?\n\n(f) Feet\n(v) Varas\n(r) Rods\n(c) Chains\n(p) Poles\n(y) Yards");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let unit_choice = input.trim().to_lowercase();

    let mut lat: f64 = 0.0;
    let mut long: f64 = 0.0;
    let lines = data.lines();

    if let Some(line) = lines.clone().next() {
        let coords: Vec<&str> = line.split_whitespace().collect();
        lat = coords[0].parse().unwrap();
        long = coords[1].parse().unwrap();
    }

    if lat < 25.0 || lat > 50.0 || long > -60.0 || long < -125.0 {
        println!("Point of Beginning is outside the Continental U.S.");
        return Ok(());
    }

    if long > 0.0 {
        long = -long;
    }

    let xratio = get_long_ratio(lat);
    let yratio = get_lat_ratio(lat);
    let mut coordinates = vec![(long, lat)];

    for line in lines.skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            eprintln!("Skipping invalid line: {}", line);
            continue;
        }

        let ns_bearing = parts[0];
        let degrees: f64 = parts[1].parse().unwrap();
        let minutes: f64 = parts[2].parse().unwrap();
        let seconds: f64 = parts[3].parse().unwrap();
        let ew_bearing = parts[4];
        let distance: f64 = parts[5].parse().unwrap();

        let decimal_degrees = degrees + (minutes / 60.0) + (seconds / 3600.0);
        let azimuth_degrees = match (ns_bearing, ew_bearing) {
            ("N", "E") | ("n", "e") => decimal_degrees,
            ("N", "W") | ("n", "w") => 360.0 - decimal_degrees,
            ("S", "E") | ("s", "e") => 180.0 - decimal_degrees,
            ("S", "W") | ("s", "w") => 180.0 + decimal_degrees,
            _ => 0.0,
        };

        let a_radians = azimuth_degrees.to_radians();
        let hypotenuse_in_feet = match unit_choice.as_str() {
            "f" => distance,
            "v" => distance * 2.77778333333,
            "r" => distance * 16.5,
            "c" => distance * 66.0,
            "p" => distance * 16.5,
            "y" => distance * 3.0,
            _ => 0.0,
        };

        let x_add = a_radians.sin() * hypotenuse_in_feet * xratio;
        let y_add = a_radians.cos() * hypotenuse_in_feet * yratio;

        let last_coord = coordinates.last().unwrap();
        coordinates.push((last_coord.0 + x_add, last_coord.1 + y_add));
    }

    // Generate output filenames
    let kml_output_path = format!("{}.kml", base_filename);
    let mut output_file = File::create(&kml_output_path)?;

    writeln!(output_file, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(output_file, "<kml xmlns=\"http://www.opengis.net/kml/2.2\">")?;
    writeln!(output_file, "<Document>")?;
    writeln!(output_file, "<Placemark>")?;
    writeln!(output_file, "<Polygon>")?;
    writeln!(output_file, "<outerBoundaryIs>")?;
    writeln!(output_file, "<LinearRing>")?;
    writeln!(output_file, "<coordinates>")?;

    for (long, lat) in &coordinates {
        writeln!(output_file, "{},{},0", long, lat)?;
    }

    writeln!(output_file, "</coordinates>")?;
    writeln!(output_file, "</LinearRing>")?;
    writeln!(output_file, "</outerBoundaryIs>")?;
    writeln!(output_file, "</Polygon>")?;
    writeln!(output_file, "</Placemark>")?;
    writeln!(output_file, "</Document>")?;
    writeln!(output_file, "</kml>")?;

    println!("KML file generated successfully: {}", kml_output_path);

    // Call function to write GeoJSON
    write_geojson(&base_filename, &coordinates)?;

    Ok(())
}
