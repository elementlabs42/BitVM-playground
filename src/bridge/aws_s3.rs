use aws_sdk_s3::{
    config::{Credentials, Region},
    error::SdkError,
    operation::put_object::{PutObjectError, PutObjectOutput},
    primitives::ByteStream,
    types::Error,
    Client, Config,
};
use dotenv;
pub struct AwsS3 {
    client: Client,
    bucket: String,
}

impl AwsS3 {
    pub fn new() -> Self {
        dotenv::dotenv().ok();
        let access_key = dotenv::var("BRIDGE_AWS_ACCESS_KEY_ID").unwrap();
        let secret = dotenv::var("BRIDGE_AWS_SECRET_ACCESS_KEY").unwrap();
        let region = dotenv::var("BRIDGE_AWS_REGION").unwrap();
        let bucket = dotenv::var("BRIDGE_AWS_BUCKET").unwrap();

        let credentials = Credentials::new(access_key, secret, None, None, "Bridge");

        let config = Config::builder()
            .credentials_provider(credentials)
            .region(Region::new(region))
            .behavior_version_latest()
            .build();

        Self {
            client: Client::from_conf(config),
            bucket,
        }
    }

    // pub async fetch_latest(&self) {
    //   // TODO
    //   // use regexp to find all matching files
    //   // sort by date
    //   // fetch newest one
    //   // validate
    //   // if invalid, go to next one
    // }

    pub async fn list_objects(&self) -> Result<(), Error> {
        let mut response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .max_keys(10) // In this example, go 10 at a time.
            .into_paginator()
            .send();

        while let Some(result) = response.next().await {
            match result {
                Ok(output) => {
                    for object in output.contents() {
                        println!(" - {}", object.key().unwrap_or("Unknown"));
                    }
                }
                Err(err) => {
                    eprintln!("{err:?}")
                }
            }
        }

        Ok(())
    }

    pub async fn get_object(&self, key: &str) -> Result<usize, Error> {
        let mut object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .unwrap();

        let mut byte_count = 0_usize;
        while let Some(bytes) = object.body.try_next().await.unwrap() {
            let bytes_len = bytes.len();
            // file.write_all(&bytes)?;
            println!("Intermediate write of {bytes_len}");
            byte_count += bytes_len;
        }

        Ok(byte_count)
    }

    pub async fn upload_object(
        &self,
        key: &str,
        data: ByteStream,
    ) -> Result<PutObjectOutput, SdkError<PutObjectError>> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data)
            .send()
            .await
    }
}
