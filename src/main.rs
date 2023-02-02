//! Discover Bluetooth devices and list them.

use bluer::{Adapter, AdapterEvent, Address, DeviceEvent};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use std::{collections::HashSet, env};

use phf::phf_map;
use chrono::{DateTime, Utc}; // 0.4.15
use std::time::SystemTime;



// #[derive(Clone)]
// pub enum Keyword {
//     Loop,
//     Continue,
//     Break,
//     Fn,
//     Extern,
// }

// static KEYWORDS: phf::Map<&'static str, Keyword> = phf_map! {
//     "loop" => Keyword::Loop,
//     "continue" => Keyword::Continue,
//     "break" => Keyword::Break,
//     "fn" => Keyword::Fn,
//     "extern" => Keyword::Extern,
// };

static MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "ATC_5791C5" => "dining_room",
    "ATC_BB91AD" => "chanel_bedroom",
    "ATC_B0D3D5" => "basement",
    "ATC_40FA49" => "emmanuel_bedroom",
    "ATC_D3CFDF" => "jhonuel_bedroom",
    "ATC_E2D32F" => "guest_bedroom",
    "ATC_D666B3" => "main_bedroom",
    "ATC_7AB84F" => "kitchen",
    "ATC_DC35FA" => "fireplace_room",
    "ATC_309577" => "garage",
    "ATC_24BC21" => "outside",

};

fn get_device_map(device_id: &str) -> Option<&str> {
    return MAP.get(device_id).cloned()
}

async fn query_device(adapter: &Adapter, addr: Address) -> bluer::Result<()> {
    let device = adapter.device(addr)?;

    let device_name = device.name().await?;
    let assigned_name = match device_name {
        Some(x) => {
            if !x.contains("ATC_") {
                return Ok(())        
            }
            x
       
        }
        None => {
            return Ok(())
        }
    };

         
    
    let mapped_name: &str = match get_device_map(&assigned_name) {
                    Some(name) => name,
                    None => "unknown"
                };
    
    
    if let Some(it) = device.service_data().await? {
        for (uuid, value) in it {

            //println!("\nUUID: {} / LEN: {}\n", key, value.len());
            let temp_cel = ((value[6] as u16) << 8) | value[7] as u16;
            let temperature_far = temp_cel as f32 * 0.18 + 32.0;
            let hum_pct = value[8];
            let batt = value[9];
            
            let now = SystemTime::now();
            let now: DateTime<Utc> = now.into();
            let now = now.to_rfc3339();
            /*
            println!("datetime: {}", now); 
            println!("UUID: {}", uuid); 
            println!("TEMP C: {}", temp_cel);
            println!("TEMP F: {}", temperature_far);
            println!("HUM%: {}", hum_pct);
            println!("Batt: {}", batt);
            */

            println!("{},{},{},{},{},{},{},{}", now, uuid, assigned_name, mapped_name, temp_cel, temperature_far, hum_pct, batt);
        }
    
    };

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
    
    Ok(())
}
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

                        // println!("Device added: {}", addr);
                        // let res = if all_properties {
                        //     query_all_device_properties(&adapter, addr).await
                        // } else {
                        //     query_device(&adapter, addr).await
                        // };

                        let res = query_device(&adapter, addr).await;
                        


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
