use bumpalo::Bump;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufRead;
use std::path::Path;
use std::time::Instant;

const INPUT_FILE_PATH: &'static str = "/dev/shm/measurements.txt";
const OUTPUT_FILE_PATH: &'static str = "summary.txt";
// Size of the buffer that will hold the binary data.
// Here I chose a 100MiB buffer.
const FILE_BUFF_SIZE: usize = 1024 * 1024 * 100;
const CITY_NAME_BUFF_SIZE: usize = 1024 * 1024 * 100;

fn main() {
    // create a new path to the file
    let path = Path::new(INPUT_FILE_PATH);

    // test if the file exists
    if !path.exists() {
        panic!("The provided file does not exist.");
    }

    // open the file
    let file_handle = File::open(path).expect("Could not open file.");

    // We create a buffer reader.
    // This is a performance optimization, as it allows us to read the file in chunks,
    // instead of doing a syscall for each line, without compromising on code readability.
    // Technically this isn't needed, because our file is on `/dev/shm`, but I let it here
    // to make sure the code still works properly if the file is on a regular disk.
    let mut reader = std::io::BufReader::with_capacity(FILE_BUFF_SIZE, file_handle);

    // create our data holding structures
    let city_name_buffer = &mut Bump::with_capacity(CITY_NAME_BUFF_SIZE);
    let mut data_summary: HashMap<&str, CityData> = HashMap::new();
    let mut line_counter: u64 = 0;
    let mut line_buffer = String::with_capacity(100); // a single allocation for the whole program

    // iterate over the lines
    let start_time = Instant::now();
    loop {
        // clear the line buffer
        line_buffer.clear();

        // read a line into the buffer
        let bytes_read = reader
            .read_line(&mut line_buffer)
            .expect("Could not read line.");

        // exit the loop if we reached the end of the file
        if bytes_read == 0 {
            break;
        }

        // process the line & append to the data summary
        process_line(city_name_buffer, &mut data_summary, &line_buffer);

        // increment the line counter
        line_counter += 1;
    }
    let duration = start_time.elapsed();

    // save the summary to a String
    let mut summary_string = String::with_capacity(1024 * 1024 * 1024); // a single allocation for the whole program
    summary_string.push('{');
    data_summary.values().for_each(|city_data| {
        city_data.summary(&mut summary_string);
        summary_string.push(',');
        summary_string.push(' ');
    });
    summary_string.pop(); // remove the last space
    summary_string.pop(); // remove the last comma
    summary_string.push('}');

    // write the summary to a file
    fs::write(OUTPUT_FILE_PATH, summary_string).expect("Could not write summary to file.");

    // print the number of lines processed and the time it took
    println!(
        "The program read {} lines in {}ms.",
        line_counter,
        duration.as_millis()
    );
}

/// Data structure to hold a single city data
/// Instead of using floats, we use integers to represent the temperature,
/// as we know that the temperature is given in 0.1° increments
struct CityData<'a> {
    city_name: &'a str,
    minimum_temperature: i64,
    maximum_temperature: i64,
    temperatures_sum: i64,
    data_points: i64,
}

impl<'a> CityData<'a> {
    /// Implement a summary function for the CityData struct
    /// We don't want heap allocation, so we use a mutable string reference on which we append the summary.
    fn summary(&self, summary_string: &mut String) {
        summary_string.push_str(self.city_name);
        summary_string.push('=');
        int_to_temperature::<10>(summary_string, self.minimum_temperature);
        summary_string.push('/');
        int_to_temperature::<10>(
            summary_string,
            // there probably are some rounding errors here, but it's beside the point of the challenge
            self.temperatures_sum / self.data_points,
        );
        summary_string.push('/');
        int_to_temperature::<10>(summary_string, self.maximum_temperature);
    }
}

/// Process a line of the file and append the data to the summary.
/// This function is optimized to avoid heap allocation.
fn process_line<'a: 'b, 'b>(
    city_name_buffer: &'a Bump,
    data_summary: &'_ mut HashMap<&'b str, CityData<'b>>,
    line: &'_ str,
) {
    // split the line into city and temperature
    let (city_name, raw_temperature) = line.split_once(';').expect("Invalid line format.");

    // convert the temperature to an integer
    let temperature = temperature_to_int(raw_temperature);

    // get the city data
    let city_data = match data_summary.get_mut(city_name) {
        Some(city_data) => city_data,
        None => {
            let longlived_city_name: &str = city_name_buffer.alloc_str(city_name);
            data_summary.insert(
                longlived_city_name,
                CityData {
                    city_name: longlived_city_name,
                    minimum_temperature: temperature,
                    maximum_temperature: temperature,
                    temperatures_sum: 0,
                    data_points: 0,
                },
            );
            data_summary
                .get_mut(city_name)
                .expect("Could not insert city data.")
        }
    };

    // update the city data
    if temperature < city_data.minimum_temperature {
        city_data.minimum_temperature = temperature;
    }
    if city_data.maximum_temperature < temperature {
        city_data.maximum_temperature = temperature;
    }
    city_data.temperatures_sum += temperature;
    city_data.data_points += 1;
}

/// Convert a raw temperature string to an integer quickly.
/// We take advantage of the fact that the temperature is given in 0.1° increments.
fn temperature_to_int(raw_temperature: &str) -> i64 {
    let mut temperature = 0;
    let mut is_negative = false;

    if raw_temperature.is_empty() {
        panic!("Empty temperature string.")
    }

    for c in raw_temperature.chars() {
        match c {
            '0'..='9' => {
                temperature *= 10;
                temperature += (c as u8 - b'0') as i64;
            }
            '-' => is_negative = true,
            '.' | '\n' => continue,
            _ => unreachable!("Invalid character in temperature : {}.", c),
        }
    }

    if is_negative {
        -temperature
    } else {
        temperature
    }
}

/// Convert an integer to a temperature string quickly.
/// /!\ The temperature must not be more than TEMP_BUFF_SIZE digits long.
fn int_to_temperature<const TEMP_BUFF_SIZE: usize>(
    summary_string: &mut String,
    mut temperature: i64,
) {
    let mut digits = [0u8; TEMP_BUFF_SIZE]; // this happens on the stack

    if temperature == 0 {
        summary_string.push_str("0.0");
        return;
    }

    if temperature < 0 {
        summary_string.push('-');
        temperature = -temperature;
    }

    let mut index = TEMP_BUFF_SIZE - 1;
    while temperature > 0 {
        digits[index] = (temperature % 10) as u8;
        index -= 1;
        temperature /= 10;
    }

    for digit_index in index + 1..TEMP_BUFF_SIZE {
        summary_string.push((b'0' + digits[digit_index]) as char);
        if digit_index == 8 {
            summary_string.push('.');
        }
    }
}
