mod latlong_ratios;  // 12-26-2024 DC:  Import the file where I moved my modules.

use latlong_ratios::get_lat_ratio;  //12-26-2024 DC:  Bring my functions into scope
use latlong_ratios::get_long_ratio; //12-26-2024 DC:  Bring my functions into scope

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::env;
use std::fs;
use std::process;

fn main() -> io::Result<()> {
    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check if the filename argument is provided
    if args.len() < 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        process::exit(1);
    }

    // Get the filename from the arguments
    let filename = &args[1];

    // Read the file contents
    let data = fs::read_to_string(filename)
        .expect("Unable to read file");

    // Print the file contents
    println!("File Contents:\n{}", data);

    //  const PI: f64 = 3.14159265358979323;

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
        println!("Your data appears to have a Point of Beginning that is outside the Continental \nUnited States.");
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
    
        // Check if parts has at least 6 elements
        if parts.len() < 6 {
            eprintln!("Skipping invalid line: {}", line);
            continue; // Skip this line and proceed with the next iteration
        }
    
        let ns_bearing = parts[0];
        let degrees: f64 = parts[1].parse().unwrap();
        let minutes: f64 = parts[2].parse().unwrap();
        let seconds: f64 = parts[3].parse().unwrap();
        let ew_bearing = parts[4];
        let distance: f64 = parts[5].parse().unwrap();
    
        let decimal_degrees = degrees + (minutes / 60.0) + (seconds / 3600.0);
    
        let azimuth_degrees = match (ns_bearing, ew_bearing) {
            ("N", "E") | ("n", "e") | ("8", "6") => decimal_degrees,
            ("N", "W") | ("n", "w") | ("8", "4") => 360.0 - decimal_degrees,
            ("S", "E") | ("s", "e") | ("2", "6") => 180.0 - decimal_degrees,
            ("S", "W") | ("s", "w") | ("2", "4") => 180.0 + decimal_degrees,
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
    

    let output_file_path = Path::new("Generated.kml");
    let mut output_file = File::create(output_file_path)?;

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

    println!("KML file generated successfully!");
    Ok(())
}
