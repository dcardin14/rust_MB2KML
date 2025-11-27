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
////////////////////////////

fn to_feet(value: f64, unit_choice: &str) -> f64 {
    match unit_choice {
        "f" => value,
        "v" => value * 2.77778333333,
        "r" => value * 16.5,
        "c" => value * 66.0,
        "p" => value * 16.5,
        "y" => value * 3.0,
        _ => 0.0,
    }
}
////////////////////////////



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
    
    ////////////////////////////////////////////////
    let mut coordinates = vec![(long, lat)];
    let mut last_azimuth_degrees: Option<f64> = None;
    ////////////////////////////////////////////////
    
    for line in lines.skip(1) {
    let line = line.trim();
    if line.is_empty() {
        continue;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        continue;
    }

    // 1) Curve lines: CRV L/R RADIUS D M S
    if parts[0].eq_ignore_ascii_case("CRV") {
        if parts.len() < 6 {
            eprintln!("Skipping invalid curve line (need CRV L/R RADIUS D M S): {}", line);
            continue;
        }

        // You must have a previous straight-line segment to define the tangent
        let start_azimuth_degrees = match last_azimuth_degrees {
            Some(a) => a,
            None => {
                eprintln!(
                    "Curve line '{}' has no preceding tangent (no azimuth). Skipping.",
                    line
                );
                continue;
            }
        };

        let side = parts[1];
        let radius_raw: f64 = parts[2].parse().unwrap_or_else(|_| {
            eprintln!("Invalid radius in curve line: {}", line);
            0.0
        });
        let d: f64 = parts[3].parse().unwrap_or_else(|_| {
            eprintln!("Invalid degrees in curve line: {}", line);
            0.0
        });
        let m: f64 = parts[4].parse().unwrap_or_else(|_| {
            eprintln!("Invalid minutes in curve line: {}", line);
            0.0
        });
        let s: f64 = parts[5].parse().unwrap_or_else(|_| {
            eprintln!("Invalid seconds in curve line: {}", line);
            0.0
        });

        let delta_degrees = d + (m / 60.0) + (s / 3600.0);
        if delta_degrees <= 0.0 || radius_raw <= 0.0 {
            eprintln!("Skipping degenerate curve line: {}", line);
            continue;
        }

        // Convert radius to feet using same unit system as distances
        let radius_feet = to_feet(radius_raw, &unit_choice);

        // Total central angle in radians
        let delta_radians = delta_degrees.to_radians();

        // Decide how many small segments to approximate the curve with
        let segments = 16; // you can tweak this (more segments = smoother curve)
        let delta_per_seg_deg = delta_degrees / segments as f64;
        let delta_per_seg_rad = delta_radians / segments as f64;

        // Arc length per segment, approximate chord length the same
        let arc_len_per_seg_feet = radius_feet * delta_per_seg_rad;

        // Direction we increment azimuth: left = +, right = -
        let sign = match side {
            "L" | "l" => 1.0_f64,
            "R" | "r" => -1.0_f64,
            _ => {
                eprintln!("Unknown curve side (use L or R): {}", line);
                continue;
            }
        };

        let mut current_azimuth_deg = start_azimuth_degrees;
        let mut last_coord = *coordinates.last().unwrap();

        for _ in 0..segments {
            // Advance azimuth along the curve
            current_azimuth_deg += sign * delta_per_seg_deg;
            let a_radians = current_azimuth_deg.to_radians();

            // Move along this small segment
            let x_add = a_radians.sin() * arc_len_per_seg_feet * xratio;
            let y_add = a_radians.cos() * arc_len_per_seg_feet * yratio;

            last_coord = (last_coord.0 + x_add, last_coord.1 + y_add);
            coordinates.push(last_coord);
        }

        // After finishing the curve, remember final azimuth for next call
        last_azimuth_degrees = Some(current_azimuth_deg);
        continue;
    }

    // 2) Straight lines (existing behavior)
    if parts.len() < 6 {
        eprintln!("Skipping invalid line: {}", line);
        continue;
    }

    let ns_bearing = parts[0];
    let degrees: f64 = parts[1].parse().unwrap_or_else(|_| {
        eprintln!("Invalid degree value in line: {}", line);
        0.0
    });
    let minutes: f64 = parts[2].parse().unwrap_or_else(|_| {
        eprintln!("Invalid minute value in line: {}", line);
        0.0
    });
    let seconds: f64 = parts[3].parse().unwrap_or_else(|_| {
        eprintln!("Invalid second value in line: {}", line);
        0.0
    });
    let ew_bearing = parts[4];
    let distance: f64 = parts[5].parse().unwrap_or_else(|_| {
        eprintln!("Invalid distance in line: {}", line);
        0.0
    });

    let decimal_degrees = degrees + (minutes / 60.0) + (seconds / 3600.0);
    let azimuth_degrees = match (ns_bearing, ew_bearing) {
        ("N", "E") | ("n", "e") => decimal_degrees,
        ("N", "W") | ("n", "w") => 360.0 - decimal_degrees,
        ("S", "E") | ("s", "e") => 180.0 - decimal_degrees,
        ("S", "W") | ("s", "w") => 180.0 + decimal_degrees,
        _ => {
            eprintln!("Invalid bearing combination in line: {}", line);
            0.0
        }
    };

    let a_radians = azimuth_degrees.to_radians();
    let hypotenuse_in_feet = to_feet(distance, &unit_choice);

    let x_add = a_radians.sin() * hypotenuse_in_feet * xratio;
    let y_add = a_radians.cos() * hypotenuse_in_feet * yratio;

    let last_coord = coordinates.last().unwrap();
    coordinates.push((last_coord.0 + x_add, last_coord.1 + y_add));

    // Remember azimuth for possible following curve
    last_azimuth_degrees = Some(azimuth_degrees);
}
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
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
