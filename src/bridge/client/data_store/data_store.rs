use once_cell::sync::Lazy;
use regex::Regex;
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use super::base::DataStoreDriver;
use super::{
    aws_s3::AwsS3,
    ftp::{ftp::Ftp, ftps::Ftps},
    sftp::Sftp,
};

static CLIENT_MISSING_CREDENTIALS_ERROR: &str =
    "Bridge client is missing AWS S3, FTP, FTPS, or SFTP credentials";

static CLIENT_DATA_SUFFIX: &str = "-bridge-client-data.json";
static CLIENT_DATA_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(&format!(r"(\d{{13}}){}", CLIENT_DATA_SUFFIX)).unwrap());

pub struct DataStore {
    aws_s3: Option<AwsS3>,
    ftp: Option<Ftp>,
    ftps: Option<Ftps>,
    sftp: Option<Sftp>,
}

impl DataStore {
    pub fn new() -> Self {
        Self {
            aws_s3: AwsS3::new(),
            ftp: Ftp::new(),
            ftps: Ftps::new(),
            sftp: Sftp::new(),
        }
    }

    pub async fn fetch_latest_data(&self) -> Result<Option<String>, String> {
        match self.get_driver() {
            Ok(driver) => {
                let keys = driver.list_objects().await;

                if keys.is_ok() {
                    let mut data_keys: Vec<String> = keys
                        .unwrap()
                        .iter()
                        .filter(|key| CLIENT_DATA_REGEX.is_match(key))
                        .cloned()
                        .collect();
                    data_keys.sort_by(|x, y| {
                        if x < y {
                            return Ordering::Less;
                        }
                        return Ordering::Greater;
                    });

                    while let Some(key) = data_keys.pop() {
                        let json = driver.fetch_json(&key).await;
                        if json.is_ok() {
                            println!("Fetched latest data file: {}", key);
                            return Ok(Some(json.unwrap()));
                        } else {
                            eprintln!("Unable to fetch json: {}", json.unwrap_err());
                        }
                    }
                } else {
                    eprintln!("Unable to list objects: {}", keys.unwrap_err());
                }

                println!("No data file found");
                Ok(None)
            }
            Err(err) => Err(err.to_string()),
        }
    }

    pub async fn write_data(&self, json: String) -> Result<String, String> {
        match self.get_driver() {
            Ok(driver) => {
                let time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let key = format!("{}{}", time, CLIENT_DATA_SUFFIX);

                let response = driver.upload_json(&key, json).await;

                match response {
                    Ok(_) => Ok(key),
                    Err(err) => Err(format!("Failed to save data file: {}", err)),
                }
            }
            Err(err) => Err(err.to_string()),
        }
    }

    fn get_driver(&self) -> Result<&dyn DataStoreDriver, &str> {
        if self.aws_s3.is_some() {
            return Ok(self.aws_s3.as_ref().unwrap());
        } else if self.ftp.is_some() {
            return Ok(self.ftp.as_ref().unwrap());
        } else if self.ftps.is_some() {
            return Ok(self.ftps.as_ref().unwrap());
        } else if self.sftp.is_some() {
            return Ok(self.sftp.as_ref().unwrap());
        } else {
            Err(CLIENT_MISSING_CREDENTIALS_ERROR)
        }
    }
}
