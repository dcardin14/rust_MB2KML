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
///////////////////////////////////////////////////////////////////////////
fn print_usage(program: &str) {
    eprintln!(
"Usage:
  {prog} <input_file>

Description:
  {prog} reads a metes-and-bounds text file and generates a closed polygon
  in both KML (.kml) and GeoJSON (.geojson) formats.

  The input file must have:
    • First line: latitude and longitude of the Point of Beginning (POB),
      in decimal degrees:  LAT LON
    • Subsequent lines: straight-line calls and/or curves.

Supported distance units:
  On startup, the program prompts for units:
    f  Feet
    v  Varas
    r  Rods
    c  Chains
    p  Poles
    y  Yards

  All distances in the file are interpreted in that unit system and
  internally converted to feet.

Input format:

  1) Point of Beginning (first line)
       LAT LON
     Example:
       29.7604 -95.3698

  2) Straight lines (quadrant bearing + distance)
       N|S  D  M  S  E|W  DIST
     Example:
       N 03 02 00 E 120.43

  3) Curves by Radius + Delta (central angle)
       CRV  L|R  RADIUS  D  M  S
     Example:
       CRV L 1992.0 34 26 06

     Here Delta (D M S) is the central angle of the curve. The tangent
     direction at the beginning of the curve is inherited from the
     previous straight segment.

  4) Curves by Radius + Arc Length
       CRV_RL  L|R  RADIUS  ARC_LENGTH
     Example:
       CRV_RL R 1500.0 800.0

     The central angle Δ is computed from:
       Δ = ARC_LENGTH / RADIUS

     The curve is tangent to the previous line, and the direction of
     curvature (left/right) is given by L or R.

  5) Curves by Radius + Delta + Chord
       CRV_RDC  L|R  RADIUS  D  M  S  CH_NS  CH_D  CH_M  CH_S  CH_EW  CHORD_DIST
     Example:
       CRV_RDC L 1992.0 34 26 06 N 45 00 00 E 1600.00

     This form specifies:
       • Curve geometry: radius + central angle (Delta)
       • Orientation: chord bearing (CH_NS / CH_D / CH_M / CH_S / CH_EW)
       • Check distance: chord length (CHORD_DIST)

     The tangent at the Point of Curvature (PC) is derived from the
     chord bearing and Delta.

Output:
  • <input_stem>.kml     – polygon as a KML file
  • <input_stem>.geojson – polygon as a GeoJSON file
  The polygon is explicitly closed by repeating the POB at the end.

Examples:
  {prog} tract1.txt
  {prog} my_legal_description.txt
",
        prog = program
    );
}

///////////////////////////////////////////////////////////////////////////
// Convert quadrant bearing (e.g. N 45 00 00 E) to azimuth in degrees
fn quadrant_to_azimuth(
    dir_ns: &str,
    degrees: f64,
    minutes: f64,
    seconds: f64,
    dir_ew: &str
) -> Option<f64> {
    let angle = degrees + minutes / 60.0 + seconds / 3600.0;

    let ns = dir_ns.to_uppercase();
    let ew = dir_ew.to_uppercase();

    match (ns.as_str(), ew.as_str()) {
        ("N", "E") => Some(angle),
        ("N", "W") => Some(360.0 - angle),
        ("S", "E") => Some(180.0 - angle),
        ("S", "W") => Some(180.0 + angle),
        _ => None,
    }
}


///////////////////////////////////////////////////////////////////////////



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
    let program = &args[0];

    // No arguments → print usage
    if args.len() < 2 {
        print_usage(program);
        return Ok(());
    }

    let arg1 = &args[1];

    // Handle -h / --help
    if arg1 == "-h" || arg1 == "--help" {
        print_usage(program);
        return Ok(());
    }

    let filename = arg1;

    // 🔹 Recreate base_filename for KML/GeoJSON output
    let base_filename = Path::new(filename)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let data = match fs::read_to_string(filename) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Unable to read file '{}': {}", filename, e);
            process::exit(1);
        }
    };

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

    // 1a) Curves defined by Radius + Arc Length: CRV_RL L/R RADIUS ARC_LENGTH
    if parts[0].eq_ignore_ascii_case("CRV_RL") {
        if parts.len() < 4 {
            eprintln!("Skipping invalid curve line (need CRV_RL L/R RADIUS ARC_LENGTH): {}", line);
            continue;
        }

        // Need a previous straight-line call to define tangent
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
        let arc_raw: f64 = parts[3].parse().unwrap_or_else(|_| {
            eprintln!("Invalid arc length in curve line: {}", line);
            0.0
        });

        if radius_raw <= 0.0 || arc_raw <= 0.0 {
            eprintln!("Skipping degenerate CRV_RL line (radius/arc <= 0): {}", line);
            continue;
        }

        // Convert to feet using same unit system as straight distances
        let radius_feet = to_feet(radius_raw, &unit_choice);
        let arc_feet = to_feet(arc_raw, &unit_choice);

        if radius_feet <= 0.0 || arc_feet <= 0.0 {
            eprintln!("Skipping CRV_RL line after unit conversion: {}", line);
            continue;
        }

        // Central angle from arc length: Δ = L / R
        let delta_radians = arc_feet / radius_feet;
        let delta_degrees = delta_radians.to_degrees();

        // Decide how many small segments to approximate the curve with
        let segments = 16; // tweak as needed
        let delta_per_seg_deg = delta_degrees / segments as f64;
        let delta_per_seg_rad = delta_radians / segments as f64;

        // Arc length per segment
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
            current_azimuth_deg += sign * delta_per_seg_deg;
            let a_radians = current_azimuth_deg.to_radians();

            let x_add = a_radians.sin() * arc_len_per_seg_feet * xratio;
            let y_add = a_radians.cos() * arc_len_per_seg_feet * yratio;

            last_coord = (last_coord.0 + x_add, last_coord.1 + y_add);
            coordinates.push(last_coord);
        }

        last_azimuth_degrees = Some(current_azimuth_deg);
        continue;
    }
////////////////////////////////////////////////////////////////////////////
        // 1c) Curves defined by Radius + Delta + Chord: CRV_RDC L/R R D M S CH_NS CH_D CH_M CH_S CH_EW CH_DIST
        if parts[0].eq_ignore_ascii_case("CRV_RDC") {
            if parts.len() < 12 {
                eprintln!(
                    "Skipping invalid CRV_RDC line (need CRV_RDC L/R R D M S CH_NS CH_D CH_M CH_S CH_EW CH_DIST): {}",
                    line
                );
                continue;
            }

            let side = parts[1];

            // Radius
            let radius_raw: f64 = match parts[2].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Invalid radius in CRV_RDC: {}", line);
                    continue;
                }
            };

            // Delta (central angle) D M S
            let d: f64 = match parts[3].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Invalid delta degrees in CRV_RDC: {}", line);
                    continue;
                }
            };
            let m: f64 = match parts[4].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Invalid delta minutes in CRV_RDC: {}", line);
                    continue;
                }
            };
            let s: f64 = match parts[5].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Invalid delta seconds in CRV_RDC: {}", line);
                    continue;
                }
            };

            // Chord bearing N/S D M S E/W
            let ch_ns = parts[6];
            let ch_d: f64 = match parts[7].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Invalid chord degrees in CRV_RDC: {}", line);
                    continue;
                }
            };
            let ch_m: f64 = match parts[8].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Invalid chord minutes in CRV_RDC: {}", line);
                    continue;
                }
            };
            let ch_s: f64 = match parts[9].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Invalid chord seconds in CRV_RDC: {}", line);
                    continue;
                }
            };
            let ch_ew = parts[10];

            // Chord distance
            let chord_dist_raw: f64 = match parts[11].parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Invalid chord distance in CRV_RDC: {}", line);
                    continue;
                }
            };

            if radius_raw <= 0.0 {
                eprintln!("Invalid radius in CRV_RDC (<= 0): {}", line);
                continue;
            }

            let radius_feet = to_feet(radius_raw, &unit_choice);
            if radius_feet <= 0.0 {
                eprintln!("Radius converts to zero/negative in CRV_RDC: {}", line);
                continue;
            }

            let delta_degrees = d + m / 60.0 + s / 3600.0;
            let delta_radians = delta_degrees.to_radians();

            if delta_degrees <= 0.0 {
                eprintln!("Delta angle is <= 0 in CRV_RDC: {}", line);
                continue;
            }

            // Chord bearing to azimuth
            let chord_bearing_deg = match quadrant_to_azimuth(ch_ns, ch_d, ch_m, ch_s, ch_ew) {
                Some(a) => a,
                None => {
                    eprintln!("Invalid chord bearing in CRV_RDC: {}", line);
                    continue;
                }
            };

            // Chord distance (feet)
            let chord_dist_feet = to_feet(chord_dist_raw, &unit_choice);
            if chord_dist_feet <= 0.0 {
                eprintln!("Chord distance converts to zero/negative in CRV_RDC: {}", line);
                continue;
            }

            // Geometric chord from R and Δ: c = 2 R sin(Δ/2)
            let geom_chord_feet = 2.0 * radius_feet * (delta_radians / 2.0).sin();

            // Optional sanity check
            let diff = (geom_chord_feet - chord_dist_feet).abs();
            if geom_chord_feet > 0.0 && diff / geom_chord_feet > 0.05 {
                eprintln!(
                    "Warning: chord distance differs from R,Δ geometry by >5% in CRV_RDC.\n  Geom chord ≈ {:.3}, given chord ≈ {:.3}\n  Line: {}",
                    geom_chord_feet, chord_dist_feet, line
                );
            }

            // Direction of curvature: left = +, right = -
            let sign = match side {
                "L" | "l" => 1.0_f64,
                "R" | "r" => -1.0_f64,
                _ => {
                    eprintln!("Invalid curve side in CRV_RDC (use L or R): {}", line);
                    continue;
                }
            };

            // Tangent at PC from chord bearing and Δ:
            //  For left curve:  tangent_PC = chord - Δ/2
            //  For right curve: tangent_PC = chord + Δ/2
            let start_azimuth_deg = chord_bearing_deg - sign * (delta_degrees / 2.0);

            // Approximate with segments like CRV/CRV_RL
            let segments = 16_usize;
            let delta_per_seg_deg = delta_degrees / segments as f64;
            let delta_per_seg_rad = delta_radians / segments as f64;

            let arc_len_per_seg_feet = radius_feet * delta_per_seg_rad;

            let mut current_azimuth_deg = start_azimuth_deg;
            let mut last_coord = *coordinates.last().unwrap();

            for _ in 0..segments {
                let a_rad = current_azimuth_deg.to_radians();

                let dx = a_rad.sin() * arc_len_per_seg_feet * xratio;
                let dy = a_rad.cos() * arc_len_per_seg_feet * yratio;

                last_coord = (last_coord.0 + dx, last_coord.1 + dy);
                coordinates.push(last_coord);

                // step tangent around the curve
                current_azimuth_deg += sign * delta_per_seg_deg;
            }

            // Update tangent at PT (for any following CRV_RL / CRV)
            last_azimuth_degrees = Some(current_azimuth_deg);
            continue;
        }

////////////////////////////////////////////////////////////////////////////
    // 1b) Curves defined by Radius + Delta: CRV L/R RADIUS D M S (your existing block)
if parts[0].eq_ignore_ascii_case("CRV") {
    if parts.len() < 7 {
        eprintln!("Skipping invalid CRV line (need CRV L/R RADIUS D M S): {}", line);
        continue;
    }

    // Need previous azimuth (tangent into curve)
    let start_azimuth = match last_azimuth_degrees {
        Some(a) => a,
        None => {
            eprintln!("Curve '{}' has no previous azimuth (no tangent).", line);
            continue;
        }
    };

    let side = parts[1];

    let radius_raw: f64 = match parts[2].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid radius in CRV: {}", line);
            continue;
        }
    };

    let deg: f64 = match parts[3].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid degrees in CRV: {}", line);
            continue;
        }
    };
    let min: f64 = match parts[4].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid minutes in CRV: {}", line);
            continue;
        }
    };
    let sec: f64 = match parts[5].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid seconds in CRV: {}", line);
            continue;
        }
    };

    if radius_raw <= 0.0 {
        eprintln!("Invalid radius in CRV (<=0): {}", line);
        continue;
    }

    let radius_feet = to_feet(radius_raw, &unit_choice);
    if radius_feet <= 0.0 {
        eprintln!("Radius converts to zero in CRV: {}", line);
        continue;
    }

    // Convert D/M/S to total angle in degrees
    let delta_degrees = deg + min / 60.0 + sec / 3600.0;
    let delta_radians = delta_degrees.to_radians();

    // Direction: Left = +, Right = -
    let sign = match side {
        "L" | "l" => 1.0,
        "R" | "r" => -1.0,
        _ => {
            eprintln!("Invalid curve side in CRV (use L or R): {}", line);
            continue;
        }
    };

    // Number of curve segments for approximation
    let segments = 16;
    let delta_per_seg_deg = delta_degrees / segments as f64;
    let delta_per_seg_rad = delta_radians / segments as f64;

    // Arc length per small segment
    let arc_len_per_seg = radius_feet * delta_per_seg_rad;

    let mut current_azimuth = start_azimuth;
    let mut last_coord = *coordinates.last().unwrap();

    for _ in 0..segments {
        current_azimuth += sign * delta_per_seg_deg;
        let a_rad = current_azimuth.to_radians();

        let dx = a_rad.sin() * arc_len_per_seg * xratio;
        let dy = a_rad.cos() * arc_len_per_seg * yratio;

        last_coord = (last_coord.0 + dx, last_coord.1 + dy);
        coordinates.push(last_coord);
    }

    // The tangent changes by the full delta
    last_azimuth_degrees = Some(current_azimuth);
    continue;
}
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    }
    // Generate output filenames
    let kml_output_path = format!("{}.kml", base_filename);
    let mut output_file = File::create(&kml_output_path)?;

    writeln!(output_file, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(output_file, "<kml xmlns=\"http://www.opengis.net/kml/2.2\">")?;
    writeln!(output_file, "<Document>")?;
    writeln!(output_file, "<Style id=\"blueTransparent\">")?;
    writeln!(output_file, "  <LineStyle>")?;
    writeln!(output_file, "    <color>FF0000FF</color>")?;   // Outline: solid red (can change)
    writeln!(output_file, "    <width>2</width>")?;
    writeln!(output_file, "  </LineStyle>")?;
    writeln!(output_file, "  <PolyStyle>")?;
    writeln!(output_file, "    <color>4DFF0000</color>")?;   // Fill: BLUE @ ~30% opacity
    writeln!(output_file, "    <fill>1</fill>")?;
    writeln!(output_file, "    <outline>1</outline>")?;
    writeln!(output_file, "  </PolyStyle>")?;
    writeln!(output_file, "</Style>")?;


    writeln!(output_file, "<Placemark>")?;
    writeln!(output_file, "<styleUrl>#blueTransparent</styleUrl>")?;
    writeln!(output_file, "<altitudeMode>clampToGround</altitudeMode>")?;
    writeln!(output_file, "<Polygon>")?;
    writeln!(output_file, "<outerBoundaryIs>")?;
    writeln!(output_file, "<LinearRing>")?;
    writeln!(output_file, "<coordinates>")?;

    for (long, lat) in &coordinates {
        writeln!(output_file, "{},{},0", long, lat)?;
    }
    //Explicitly close the ring
    if let Some((first_long, first_lat)) = coordinates.first() {
    writeln!(output_file, "{},{},0", first_long, first_lat)?;
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
