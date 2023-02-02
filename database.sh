#!/usr/bin/env bash


CREATE TABLE temperature_records (
  record_id INTEGER PRIMARY KEY AUTOINCREMENT,
  record_time TEXT, 
  uuid VARCHAR(50), 
  device_name VARCHAR(20), 
  mapped_name VARCHAR(50), 
  temp_celcius FLOAT, 
  temperature_fahrenheit FLOAT, 
  humidity_percentage FLOAT, 
  batttery_level FLOAT 

);



 INSERT INTO temperature_records(record_time, uuid, device_name, mapped_name, temp_celcius, temperature_fahrenheit, humidity_percentage, batttery_level)
 SELECT * FROM temperature_records_temp;

SELECT mapped_name, avg(temperature_fahrenheit) FROM temperature_records GROUP BY 1;

SELECT date(record_time), count(*)
FROM temperature_records 
GROUP BY 1;

SELECT mapped_name, date(record_time), time(record_time),  avg(temperature_fahrenheit)
FROM temperature_records  
GROUP BY 1, 2, 3
ORDER BY 1, 2, 3 ASC
;

SELECT mapped_name, date(record_time),  strftime('%H', record_time),  avg(temperature_fahrenheit)
FROM temperature_records 
GROUP BY 1, 2, 3
ORDER BY 1, 2, 3 ASC
;

 strftime('%H', date_column