// Copyright 2020 Joyent, Inc.

use slog::Logger;
use std::time::Duration;

use reqwest::{Client, IntoUrl, Response};
// Use old-style Hyper headers until they put them back in.
use reqwest::hyper_011::header::{Accept, ContentType, Headers};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Container for the zone metadata
// XXX This structure is not as stable as the others below.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ZoneConfig {
    pub manifests: Vec<SapiManifests>,
    pub metadata: Value,
}

// In an attempt to future proof these structures as much as possible the
// Option<_> type and the serde(default) field attribute have been used in
// any case where the struct field was not part of the bucket schema at time
// of creation.  The creation of each of these buckets, and the associated
// metadata can (at the time this was written) be found in the
// sdc-sapi:/lib/server/stor/moray.js`initBuckets() function.
//
// Some fields will always be part of the response... in current code.  But
// it is much more likely that those additional fields will be removed or
// modified than it is that a field will be removed from the bucket schema
// without significant scrutiny.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SapiManifests {
    pub uuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub template: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub master: bool,
    #[serde(default)]
    pub post_cmd: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceData {
    pub uuid: String,
    pub name: String,
    pub application_uuid: String,
    pub params: Option<Value>,
    pub metadata: Option<Value>,
    #[serde(default)]
    pub master: bool,
    // TODO: add the type field, which comes with sapi v2.0.
    // In order to receive that field from sapi the "accept-version: 2" header
    // field must be specified.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstanceData {
    pub uuid: String,
    pub service_uuid: String,
    pub params: Option<Value>,
    pub metadata: Option<Value>,
    // TODO: add type field.  See above.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationData {
    pub uuid: String,
    pub name: String,
    pub owner_uuid: String,
    pub params: Option<Value>,
    pub metadata: Option<Value>,
    pub manifests: Option<Value>,
}

pub type Applications = Vec<ApplicationData>;
pub type Services = Vec<ServiceData>;
pub type Instances = Vec<InstanceData>;

/// The SAPI client
#[derive(Debug)]
pub struct SAPI {
    sapi_base_url: String,
    request_timeout: u64,
    client: Client, // reqwest client
    log: Logger,
}

impl SAPI {
    /// initialize SAPI client API
    pub fn new(sapi_base_url: &str, request_timeout: u64, log: Logger) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(request_timeout))
            .build()
            .unwrap();
        SAPI {
            sapi_base_url: sapi_base_url.into(),
            request_timeout,
            client,
            log: log.clone(),
        }
    }

    /// Retrieve the "zone" configuration by zone UUID.
    pub fn get_zone_config(&self, uuid: &str) -> Result<ZoneConfig, Box<dyn std::error::Error>> {
        let url = format!("{}/configs/{}", self.sapi_base_url.clone(), uuid);
        let zconfig: ZoneConfig = self.get(&url)?.json()?;
        Ok(zconfig)
    }

    /// Get Instance
    pub fn get_instance(
        &self,
        inst_uuid: &str,
    ) -> Result<InstanceData, Box<dyn std::error::Error>> {
        let url = format!("{}/instances/{}", self.sapi_base_url.clone(), inst_uuid);
        let instance: InstanceData = self.get(&url)?.json()?;
        Ok(instance)
    }

    /// List all instances
    pub fn list_instances(&self) -> Result<Instances, Box<dyn std::error::Error>> {
        let url = format!("{}/instances", self.sapi_base_url.clone());
        let instances: Instances = self.get(&url)?.json()?;
        Ok(instances)
    }

    pub fn list_service_instances(
        &self,
        svc_uuid: &str,
    ) -> Result<Instances, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/instances?service_uuid={}",
            self.sapi_base_url.clone(),
            svc_uuid
        );
        let instances: Instances = self.get(&url)?.json()?;
        Ok(instances)
    }

    /// List all services
    pub fn list_services(&self) -> Result<Services, Box<dyn std::error::Error>> {
        let url = format!("{}/services", self.sapi_base_url.clone());
        let sdata: Services = self.get(&url)?.json()?;
        Ok(sdata)
    }

    /// get service by UUID
    pub fn get_service(&self, uuid: &str) -> Result<ServiceData, Box<dyn std::error::Error>> {
        let url = format!("{}/services/{}", self.sapi_base_url.clone(), uuid);
        let sdata: ServiceData = self.get(&url)?.json()?;
        Ok(sdata)
    }

    pub fn get_service_by_name(&self, name: &str) -> Result<Services, Box<dyn std::error::Error>> {
        let url = format!("{}/services?name={}", self.sapi_base_url.clone(), name);
        let sdata: Services = self.get(&url)?.json()?;
        Ok(sdata)
    }

    /// create the named service under the application with the passed UUID
    pub fn create_service(
        &self,
        name: &str,
        application_uuid: &str,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let body = json!({
            "name": name,
            "application_uuid": application_uuid
        });
        let url = format!("{}/services", self.sapi_base_url.clone());
        self.post(&url, &body)
    }

    /// modify the named service with the contents of 'body'
    pub fn update_service(
        &self,
        service_uuid: &str,
        body: Value,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let url = format!("{}/services/{}", self.sapi_base_url.clone(), service_uuid);
        self.post(&url, &body)
    }

    ///
    pub fn delete_service(
        &self,
        service_uuid: &str,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let url = format!("{}/services/{}", self.sapi_base_url.clone(), service_uuid);
        self.delete(&url)
    }

    pub fn get_application_by_name(
        &self,
        name: &str,
    ) -> Result<Applications, Box<dyn std::error::Error>> {
        let url = format!("{}/applications?name={}", self.sapi_base_url.clone(), name);
        let apps: Applications = self.get(&url)?.json()?;
        Ok(apps)
    }

    pub fn list_applications(&self) -> Result<Applications, Box<dyn std::error::Error>> {
        let url = format!("{}/applications", self.sapi_base_url.clone());
        let apps: Applications = self.get(&url)?.json()?;
        Ok(apps)
    }

    pub fn get_application(
        &self,
        uuid: &str,
    ) -> Result<ApplicationData, Box<dyn std::error::Error>> {
        let url = format!("{}/applications/{}", self.sapi_base_url.clone(), uuid);

        let app: ApplicationData = self.get(&url)?.json()?;
        Ok(app)
    }

    //
    // private functions
    //
    fn default_headers(&self) -> Headers {
        let mut headers = Headers::new();

        headers.set(ContentType::json());
        headers.set(Accept::json());
        headers
    }

    /// Generic get -- results deserialized by caller
    fn get<S>(&self, url: S) -> Result<Response, Box<dyn std::error::Error>>
    where
        S: IntoUrl,
    {
        match self
            .client
            .get(url)
            .headers_011(self.default_headers())
            .send()
        {
            Ok(response) => Ok(response),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Generic post
    fn post<S>(&self, url: S, body: &Value) -> Result<Response, Box<dyn std::error::Error>>
    where
        S: IntoUrl,
    {
        let resp = self
            .client
            .post(url)
            .headers_011(self.default_headers())
            .json(&body)
            .send()?;
        Ok(resp)
    }

    /// Generic delete
    fn delete<S>(&self, url: S) -> Result<Response, Box<dyn std::error::Error>>
    where
        S: IntoUrl,
    {
        let resp = self
            .client
            .delete(url)
            .headers_011(self.default_headers())
            .send()?;
        Ok(resp)
    }
}

#[test]
fn test_services() {
    use slog::{error, info, o, Drain, Logger};
    use std::sync::Mutex;

    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let log = Logger::root(
        Mutex::new(slog_term::FullFormat::new(plain).build()).fuse(),
        o!("build-id" => "0.1.0"),
    );

    let client = SAPI::new("http://10.77.77.136", 60, log.clone());

    let s_uuid = String::from("e68592d3-5677-44ec-a5e8-cfd3652dd5be");
    let name = String::from("cheddar");
    match client.create_service(&name, &s_uuid.to_string()) {
        Ok(resp) => {
            assert_eq!(resp.status().is_success(), true);
        }
        Err(_e) => assert!(false),
    }

    match client.list_services() {
        Ok(list) => {
            assert_ne!(list.len(), 0);
        }
        Err(e) => {
            info!(log, "Error: {:?}", e);
            assert!(false)
        }
    }

    let zone_uuid = String::from("f8bf03e3-5636-4cc4-a939-bbca6b4547f0");

    match client.get_zone_config(&zone_uuid) {
        Ok(resp) => {
            assert_eq!(resp.metadata["SERVICE_NAME"], "2.moray.orbit.example.com");
        }
        Err(e) => error!(log, "error: {:?}", e),
    }
}
