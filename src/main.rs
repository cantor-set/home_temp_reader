//! Discover Bluetooth devices and list them.

use bluer::{Adapter, AdapterEvent, Address, DeviceEvent, Uuid};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use std::{collections::{HashSet, HashMap}, env};

use chrono::{DateTime, Utc}; // 0.4.15
use std::time::SystemTime;

use std::fmt;

use serde::Deserialize;
use serde_json;
use std::fs::File;
use std::io::Read;

#[derive(Deserialize, Debug, Clone)]
struct Sensor {
    name: String,
    location: String
}

struct Temperature {
    celsius: f32
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
        
        write!(f, "{},{},{},{},{},{},{}", self.date_time, self.sensor.name, self.sensor.location, self.temperature, self.temperature.to_fahrenheit(), self.humidity, self.batery_level)
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    sensors: Vec<Sensor>
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
       
        sensors.insert(s.name.clone(), Sensor {
            name:s.name.clone(), location:s.location.clone()
        });
    }); 

    sensors

}



fn read_data(sensor: Sensor, it: HashMap<Uuid, Vec<u8>>) -> Option<SensorReading> {

    match it.len() {
        1 => {
            let mut reading: Option<SensorReading> = None;
            for (_, value) in it {

                let celsius: f32 = (((value[6] as i16) << 8) | value[7] as i16) as f32 /10.0;
                let hum_pct = value[8];
                let batt = value[9];
                
                let now = SystemTime::now();
                let now: DateTime<Utc> = now.into();
        
                reading = Some(SensorReading {
                    sensor,
                    date_time: now,
                    temperature: Temperature{celsius},
                    humidity: hum_pct,
                    batery_level: batt,
                });
        
                break;
        
            }
            reading
        }
        _ => {
            None
        }
    }

    

    

    

}


fn get_sensor(sensors_map: &HashMap<String, Sensor>, device_id: &str) -> Sensor {
    match sensors_map.get(device_id) {
        Some(sensor) => {
            sensor.clone()
        }
        None => {
            Sensor{name: device_id.to_string(), location: "unknown".to_string()}
        }
    }
    
}

async fn read_atc_device(sensors_map:  &HashMap<String, Sensor>, adapter: &Adapter, addr: Address) -> bluer::Result<()> {
    let device = adapter.device(addr)?;
    let device_name = device.name().await?;
    let assigned_name = match device_name {
        Some(x) => {
            // println!("{x}");
            if !x.contains("ATC_") {
                return Ok(())        
            }
            x
       
        }
        None => {
            return Ok(())
        }
    };

    let sensor = get_sensor(sensors_map, &assigned_name);

    if let Some(it) = device.service_data().await? {
        match read_data(sensor, it) {
            Some(reading) => {
                println!("{}", reading);
            }
            _  => ()
            
        }
        


    }
    Ok(())

}

    // println!("#####################");

    // println!("    Address type:       {}", device.address_type().await?);
    // println!("    Name:               {:?}", device.name().await?);
    // println!("    Icon:               {:?}", device.icon().await?);
    // println!("    Class:              {:?}", device.class().await?);
    // println!("    UUIDs:              {:?}", device.uuids().await?.unwrap_or_default());
    // println!("    Paired:             {:?}", device.is_paired().await?);
    // println!("    Connected:          {:?}", device.is_connected().await?);
    // println!("    Trusted:            {:?}", device.is_trusted().await?);
    // println!("    Modalias:           {:?}", device.modalias().await?);
    // println!("    RSSI:               {:?}", device.rssi().await?);
    // println!("    TX power:           {:?}", device.tx_power().await?);
    // println!("    Manufacturer data:  {:?}", device.manufacturer_data().await?);
    // println!("    Service data:       {:?}", device.service_data().await?);
    
/*
async fn query_all_device_properties(adapter: &Adapter, addr: Address) -> bluer::Result<()> {
    let device = adapter.device(addr)?;
    let props = device.all_properties().await?;
    for prop in props {
        println!("    {:?}", &prop);
    }
    Ok(())
}
 */

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let with_changes = false; //env::args().any(|arg| arg == "--changes");
    //let all_properties = env::args().any(|arg| arg == "--all-properties");
    let filter_addr: HashSet<_> = env::args().filter_map(|arg| arg.parse::<Address>().ok()).collect();

    let config = read_config("./config.json");

    let sensors_map = get_sensors(&config);

    env_logger::init();
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    // println!("Discovering devices using Bluetooth adapter {}\n", adapter.name());
    adapter.set_powered(true).await?;

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    let mut all_change_events = SelectAll::new();

    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(addr) => {
                        if !filter_addr.is_empty() && !filter_addr.contains(&addr) {
                            continue;
                        }


                        let res = read_atc_device(&sensors_map, &adapter, addr).await;

                        if let Err(err) = res {
                            println!("    Error: {}", &err);
                        }

                        if with_changes {
                            let device = adapter.device(addr)?;
                            let change_events = device.events().await?.map(move |evt| (addr, evt));
                            all_change_events.push(change_events);
                        }
                    }
                    /* 
                    AdapterEvent::DeviceRemoved(addr) => {
                        println!("Device removed: {}", addr);
                    }

                    AdapterEvent::PropertyChanged(property) => {
                        println!(" Property changed-->   {:?}", property);
                    }
                    */
                    _ => (),
                }
                //println!();
            }
            Some((addr, DeviceEvent::PropertyChanged(property))) = all_change_events.next() => {
                println!("Device changed: {}", addr);
                println!("    {:?}", property);
            }
            else => break
        }
    }

    Ok(())
}
