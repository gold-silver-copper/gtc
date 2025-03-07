use chrono::{NaiveTime, TimeDelta};
use csv::{Reader, ReaderBuilder, Writer};
use geo::{Distance, Haversine, Point};
use geojson::{Feature, FeatureCollection, GeoJson, Value};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{create_dir, File};
use std::io::Write;

enum Provider {
    MTS,
    NCTD,
}

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
type HeadwayMap = HashMap<String, HashSet<NaiveTime>>;
fn haversine_distance(p1: &StopLocation, p2: &BlockLocation) -> f64 {
    let point1 = Point::new(p1.lng, p1.lat);
    let point2 = Point::new(p2.lng, p2.lat);
    let dist = Haversine::distance(point1, point2); //output is in meters
    (dist * 0.00062137) // convert to miles
}

fn main() -> Result<(), Box<dyn Error>> {
    // create_centroids_csv();
    analysis();

    Ok(())
}

fn finalize_blocks(allowed_ids: HashSet<String>) -> Result<(), Box<dyn Error>> {
    let input_file = "Census_Blocks.csv";
    let output_file = "filtered_blocks_2.csv";

    // Open CSV file
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(input_file)?;

    let headers = rdr.headers()?.clone();
    let mut wtr = Writer::from_path(output_file)?;
    wtr.write_record(&headers)?;

    for result in rdr.records() {
        let record = result?;
        if let Some(id) = record.get(2) {
            // Assuming ID is in the third column
            if allowed_ids.contains(id) {
                wtr.write_record(&record)?;
            }
        }
    }

    wtr.flush()?;
    println!("Filtered CSV saved to {}", output_file);
    Ok(())
}

fn analysis() -> Result<(), Box<dyn Error>> {
    let mut transit_stops = read_transit_stops().unwrap();

    let blocks = read_centroids_csv().unwrap();
    let mut blocks_within_range = HashSet::new();

    let mut results = Vec::new();
    let mut total_population = 1;
    let mut pop_within_half_mile = 0;
    let mut nctd_headways = read_transit_stops_with_headway(Provider::NCTD).unwrap();
    let mut mts_headways = read_transit_stops_with_headway(Provider::MTS).unwrap();
    nctd_headways.append(&mut mts_headways);
    transit_stops.retain(|x| nctd_headways.contains(&x.id));
    println!("ITERATION STARTED");

    for block in &blocks {
        // break;
        let mut closest_stop: Option<String> = None;
        let mut min_distance = 0.5;
        total_population += block.population;
        for transit_stop in &transit_stops {
            let distance = haversine_distance(transit_stop, block);

            if distance < min_distance {
                min_distance = distance;
                closest_stop = Some(transit_stop.id.clone());
            }
        }
        if let Some(stop) = closest_stop {
            let resultik = (block.id.clone(), stop.clone(), min_distance);
            blocks_within_range.insert(block.id.clone());
            pop_within_half_mile += block.population;
            //println!("resultik is {:#?}", resultik);
            results.push(resultik);
        }

        println!("total population is {:#?}", total_population);
        println!("population within half mile is {:#?}", pop_within_half_mile);
        let ratio = pop_within_half_mile as f64 / total_population as f64;
        println!("ratio is {:#?}", ratio);
    }
    println!("FINAL total population is {:#?}", total_population);
    println!(
        "FINAL population within half mile is {:#?}",
        pop_within_half_mile
    );
    let ratio = pop_within_half_mile as f64 / total_population as f64;
    println!("FINAL ratio is {:#?}", ratio);
    finalize_blocks(blocks_within_range);
    Ok(())
}

fn read_transit_stops_with_headway(provider: Provider) -> Result<Vec<String>, Box<dyn Error>> {
    let pathik = match provider {
        Provider::MTS => "google_transit/stop_times.txt",
        Provider::NCTD => "gtfs/stop_times.txt",
    };
    let mut reader = Reader::from_path(pathik)?;
    let mut headway_map = HeadwayMap::new();
    let format = "%H:%M:%S";

    let prefixik = match provider {
        Provider::MTS => "MTS_",
        Provider::NCTD => "NCTD_",
    };

    for result in reader.records() {
        let record = result?;
        let mut id = record.get(3).unwrap_or("").to_string();
        id = format!("{}{}", prefixik, id);

        let mut arrival_time_pre = record.get(1).unwrap_or("").to_string();
        if arrival_time_pre.starts_with("24") {
            arrival_time_pre = format!("{}{}", "00", &arrival_time_pre[2..]); // Replace first two characters
        }
        if arrival_time_pre.starts_with("25") {
            arrival_time_pre = format!("{}{}", "01", &arrival_time_pre[2..]); // Replace first two characters
        }
        if arrival_time_pre.starts_with("26") {
            arrival_time_pre = format!("{}{}", "02", &arrival_time_pre[2..]); // Replace first two characters
        }
        if arrival_time_pre.starts_with("27") {
            arrival_time_pre = format!("{}{}", "03", &arrival_time_pre[2..]); // Replace first two characters
        }
        let arrival_time = NaiveTime::parse_from_str(&arrival_time_pre, format).unwrap();

        /*
        match arrival_time {
            Ok(_) => {}
            Err(x) => {
                println!("{:#?} {:#?}", x, arrival_time_pre);
            }
        }
        */

        match headway_map.get_mut(&id) {
            Some(pair) => {
                pair.insert(arrival_time);
            }
            None => {
                headway_map.insert(id, HashSet::from([arrival_time]));
            }
        }

        // println!("{:#?}", arrival_time);
        //  headway_map.insert(geoloc);
    }

    let mut headwayed_stops = Vec::new();

    for (key, value) in headway_map {
        if has_close_times(&value) {
            headwayed_stops.push(key.clone());
        }
    }
    // println!("{:#?}", headwayed_stops);
    Ok(headwayed_stops)
}

fn has_close_times(times: &HashSet<NaiveTime>) -> bool {
    // instant return for no headway
    //  return true;
    let mut sorted_times: Vec<_> = times.iter().collect();
    let format = "%H:%M:%S";
    let sixam = NaiveTime::parse_from_str("06:00:00", format).unwrap();
    let nineam = NaiveTime::parse_from_str("09:00:00", format).unwrap();
    let threepm = NaiveTime::parse_from_str("15:00:00", format).unwrap();
    let sixpm = NaiveTime::parse_from_str("18:00:00", format).unwrap();
    let since = NaiveTime::signed_duration_since;
    sorted_times.sort(); // Sorting ensures we only check consecutive pairs
    let min_delta = TimeDelta::minutes(0);
    let max_delta = TimeDelta::minutes(16);

    let mut close_counter = 0;

    for window in sorted_times.windows(2) {
        if let [t1, t2] = window {
            if ((sixam <= **t1) && (**t1 <= nineam)) || ((threepm <= **t1) && (**t1 <= sixpm)) {
                let diff = since(**t2, **t1);
                if (diff <= max_delta) && (min_delta <= diff) {
                    println!("{:#?}", sorted_times);
                    println!("{:#?}", [t1, t2]);
                    close_counter += 1;
                }
            }
        }
    }
    if close_counter > 23 {
        return true;
    } else {
        return false;
    }
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
        //  println!("{:#?}", geoloc);
        locations.push(geoloc);
    }

    Ok(locations)
}

fn read_centroids_csv() -> Result<Vec<BlockLocation>, Box<dyn Error>> {
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
        //  println!("{:#?}", geoloc);
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
