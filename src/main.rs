use csv::Reader;
use geo::{Distance, Haversine, Point};
use geojson::{Feature, FeatureCollection, GeoJson, Value};
use std::error::Error;
use std::fs::{create_dir, File};
use std::io::Write;

#[derive(Debug)]
struct StopLocation {
    id: String,
    lat: f64,
    lng: f64,
}
#[derive(Debug)]
struct BlockLocation {
    id: String,
    population: i64,
    lat: f64,
    lng: f64,
}
#[derive(Debug)]
struct DistanceBlock {
    id: String,
    population: i64,
    distance: f64,
}
fn haversine_distance(p1: &StopLocation, p2: &BlockLocation) -> f64 {
    let point1 = Point::new(p1.lng, p1.lat);
    let point2 = Point::new(p2.lng, p2.lat);
    let dist = Haversine::distance(point1, point2); //output is in meters
    (dist * 0.00062137) // convert to miles
}

fn main() -> Result<(), Box<dyn Error>> {
    // create_centroids_csv();
    let transit_stops = read_transit_stops().unwrap();
    let blocks = read_census_blocks().unwrap();

    let mut results = Vec::new();

    for block in &blocks {
        let mut closest_stop: Option<String> = None;
        let mut min_distance = 10.0;
        for transit_stop in &transit_stops {
            let distance = haversine_distance(transit_stop, block);

            if distance < min_distance {
                min_distance = distance;
                closest_stop = Some(transit_stop.id.clone());
            }
        }
        if let Some(stop) = closest_stop {
            let resultik = (block.id.clone(), stop.clone(), min_distance);
            println!("resultik is {:#?}", resultik);
            results.push(resultik);
        }
    }
    analysis();
    Ok(())
}

fn analysis() -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn read_transit_stops() -> Result<Vec<StopLocation>, Box<dyn Error>> {
    let mut reader = Reader::from_path("transit_stops_datasd.csv")?;
    let mut locations = Vec::new();

    for result in reader.records() {
        let record = result?;
        let id = record.get(1).unwrap_or("").to_string();
        let lat: f64 = record.get(5).unwrap_or("0").parse()?;
        let lng: f64 = record.get(6).unwrap_or("0").parse()?;
        let geoloc = StopLocation { id, lat, lng };
        println!("{:#?}", geoloc);
        locations.push(geoloc);
    }

    Ok(locations)
}

fn read_census_blocks() -> Result<Vec<BlockLocation>, Box<dyn Error>> {
    let mut reader = Reader::from_path("centroids.csv")?;
    let mut locations = Vec::new();

    for result in reader.records() {
        let record = result?;
        let id = record.get(0).unwrap_or("").to_string();
        let population: i64 = record.get(2).unwrap_or("0").parse()?;
        let lat: f64 = record.get(4).unwrap_or("0").parse()?;
        let lng: f64 = record.get(3).unwrap_or("0").parse()?;
        let geoloc = BlockLocation {
            id,
            population,
            lat,
            lng,
        };
        println!("{:#?}", geoloc);
        locations.push(geoloc);
    }

    Ok(locations)
}

fn create_centroids_csv() -> Result<(), Box<dyn Error>> {
    // Read the GeoJSON file
    let geojson_str = std::fs::read_to_string("meep.geojson")?;
    let geojson: GeoJson = geojson_str.parse()?;

    // Open CSV file for writing
    let mut csv_writer = csv::Writer::from_writer(File::create("centroids.csv")?);

    // Write header row
    csv_writer.write_record(["geoid", "name", "population", "longitude", "latitude"])?;

    if let GeoJson::FeatureCollection(FeatureCollection { features, .. }) = geojson {
        for feature in features {
            if let Some(Value::Point(coords)) = feature.geometry.as_ref().map(|g| &g.value) {
                let longitude = coords[0];
                let latitude = coords[1];
                let geoid = feature
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("GEOID"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let name = feature
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("NAME"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let population = feature
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("P0010001"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                println!("WHAT: {} ", population);

                // Write to CSV
                csv_writer.write_record(&[
                    geoid.to_string(),
                    name.to_string(),
                    population.to_string(),
                    longitude.to_string(),
                    latitude.to_string(),
                ])?;
            }
        }
    }

    csv_writer.flush()?;
    println!("Conversion complete! Output saved to centroids.csv");
    Ok(())
}
