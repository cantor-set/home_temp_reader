//! Discover Bluetooth devices and list them.
use std::thread::sleep;
use std::time;
//use bluer::{Adapter, AdapterEvent, Address, Uuid, Device};
use bluer::{AdapterEvent, Uuid,};
use futures::pin_mut;
use futures::StreamExt;
// use std::{
//     collections::{HashMap, HashSet},
//     env,
// };
use std::collections::HashMap;

use chrono::{DateTime, Utc}; // 0.4.15
use std::time::SystemTime;

use std::fmt;

use serde::Deserialize;
use serde_json;
use std::fs::File;
use std::io::Read;
//use std::sync::atomic::{AtomicUsize, Ordering};
//use std::sync::Arc;
use bluer::monitor::{Monitor, MonitorEvent, Pattern, RssiSamplingPeriod};
use tokio::task;

#[derive(Deserialize, Debug, Clone)]
struct Sensor {
    name: String,
    location: String,
}

struct Temperature {
    celsius: f32,
}

impl Temperature {
    fn to_fahrenheit(&self) -> f32 {
        // Only two digits transformation, not rounding, but truncating
        ((self.celsius * 1.8 + 32.0) * 100f32).floor() / 100.0
    }
}

impl fmt::Display for Temperature {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.2}", self.celsius)
    }
}

struct SensorReading {
    sensor: Sensor,
    date_time: DateTime<Utc>,
    temperature: Temperature,
    humidity: u8,
    batery_level: u8,
}

impl fmt::Display for SensorReading {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{},{}",
            self.date_time,
            self.sensor.name,
            self.sensor.location,
            self.temperature,
            self.temperature.to_fahrenheit(),
            self.humidity,
            self.batery_level
        )
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    sensors: Vec<Sensor>,
}

fn read_config(path: &str) -> Config {
    let mut file = File::open(path).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let config: Config = serde_json::from_str(&data).unwrap();
    config
}

fn get_sensors(config: &Config) -> HashMap<String, Sensor> {
    let mut sensors: HashMap<String, Sensor> = HashMap::new();

    let sen = config.sensors.clone();

    sen.into_iter().for_each(|s: Sensor| {
        sensors.insert(
            s.name.clone(),
            Sensor {
                name: s.name.clone(),
                location: s.location.clone(),
            },
        );
    });

    sensors
}

fn read_data(sensor: Sensor, it: HashMap<Uuid, Vec<u8>>) -> Option<SensorReading> {
    match it.len() {
        1 => {
            let mut reading: Option<SensorReading> = None;
            for (_, value) in it {
                let celsius: f32 = (((value[6] as i16) << 8) | value[7] as i16) as f32 / 10.0;
                let hum_pct = value[8];
                let batt = value[9];

                let now = SystemTime::now();
                let now: DateTime<Utc> = now.into();

                reading = Some(SensorReading {
                    sensor,
                    date_time: now,
                    temperature: Temperature { celsius },
                    humidity: hum_pct,
                    batery_level: batt,
                });
                break;
                
            }
            reading
        }
        _ => None,
    }
}

fn get_sensor(sensors_map: &HashMap<String, Sensor>, device_id: &str) -> Sensor {
    match sensors_map.get(device_id) {
        Some(sensor) => sensor.clone(),
        None => Sensor {
            name: device_id.to_string(),
            location: "unknown".to_string(),
        },
    }
}



async fn main2() -> bluer::Result<()> {
    let config = read_config("./config.json");
    let sensors_map = get_sensors(&config);
    //env_logger::init();
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    let mm = adapter.monitor().await?;
    adapter.set_powered(true).await?;
    //let sampling_time = Duration::new(0,1000);
    let mut monitor_handle = mm
        .register(Monitor {
            monitor_type: bluer::monitor::Type::OrPatterns,
            rssi_low_threshold: None,
            rssi_high_threshold: None,
            rssi_low_timeout: None,
            rssi_high_timeout: None,
            //rssi_sampling_period: Some(RssiSamplingPeriod::Period(sampling_time)),
            rssi_sampling_period: Some(RssiSamplingPeriod::All),
            patterns: Some(vec![Pattern {
                data_type: 0x09, // name
                start_position: 0x00,
                content: vec![0x41, 0x54, 0x43],}]), // ATC
            ..Default::default()
        })
        .await?;

    let mut now = time::Instant::now();    
    let one_minute = time::Duration::new(60,0);

    

    while let Some(mevt) = &monitor_handle.next().await {

        if now.elapsed() > one_minute {
            println!("Should sleep: {:?}", now);
            now = time::Instant::now();
            //sleep(one_minute);
        }

        //println!("Here -> {:?}", mevt);

        match  mevt {
            MonitorEvent::DeviceFound(d) => {
                match adapter.device(d.device) {
                    Ok(device) => {
                        
                        //let mut device_name = "";

                        //println!("Got Service DAta!!! -> {:?}", x.service_data().await?);

                        
                        let device_name = match device.name().await? {
                            Some(d_name) => {
                                d_name.clone()
                            }
                            None => "Unknown".to_string()
                                                
                        };


                        let sensor = get_sensor(&sensors_map, &device_name);
                        if let Some(it) = device.service_data().await? {
                            match read_data(sensor, it) {
                                Some(reading) => {
                                    println!("{}", reading);
                                }
                                _ => {println!("Could not read sensor data")},
                            }
                        }

                        
                    },
                    Err(y) => {
                        println!("Got Error -> {:?}", y);
                        
                    }
                }
                
            }
            _ => (),
        }
    }

    Ok(())
}



#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    env_logger::init();
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    // println!("Discovering devices using Bluetooth adapter {}\n", adapter.name());

    task::spawn(async {
        
        println!("now running on a worker thread");
        match main2().await {
            Ok(c) => {
                println!("Yes {:?}", c);
            },
            Err(_) => { println!("FAILED");}
        

        }
    });

    adapter.set_powered(true).await?;

    let device_events = adapter.discover_devices().await?;

    pin_mut!(device_events);


    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(_) => {
                    }
                    _ => (),
                }
            }
            else => {
                println!("Got here");
                break;
            }
        }
    }

    Ok(())
}
