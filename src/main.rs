use geojson::{Feature, FeatureCollection, GeoJson, Value};
use std::error::Error;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn Error>> {
    // Read the GeoJSON file
    let geojson_str = std::fs::read_to_string("meep.geojson")?;
    let geojson: GeoJson = geojson_str.parse()?;

    // Open CSV file for writing
    let mut csv_writer = csv::Writer::from_writer(File::create("output.csv")?);

    // Write header row
    csv_writer.write_record(["longitude", "latitude", "name"])?;

    if let GeoJson::FeatureCollection(FeatureCollection { features, .. }) = geojson {
        for feature in features {
            if let Some(Value::Point(coords)) = feature.geometry.as_ref().map(|g| &g.value) {
                let longitude = coords[0];
                let latitude = coords[1];
                let name = feature
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("GEOID"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Write to CSV
                csv_writer.write_record(&[
                    longitude.to_string(),
                    latitude.to_string(),
                    name.to_string(),
                ])?;
            }
        }
    }

    csv_writer.flush()?;
    println!("Conversion complete! Output saved to output.csv");
    Ok(())
}
