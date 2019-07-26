// Copyright (c) 2019 libechoexec developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Echo Structs

use {
    crate::error::ErrKind,
    getset::Setters,
    hyper::{client::HttpConnector, Body, Client, Request},
    hyper_tls::HttpsConnector,
    lazy_static::lazy_static,
    serde::ser::{Serialize as Ser, Serializer},
    serde_derive::Serialize,
    slog::{error, trace, Logger},
    slog_try::{try_error, try_trace},
    std::{collections::HashMap, error::Error, io::Write},
    tokio::runtime::Runtime,
    uuid::Uuid,
};

/// `tokio` runtime wrapper for spawning async Echo Events
#[derive(Debug)]
pub struct Spawner {
    /// The `tokio` runtime
    rt: Runtime,
    /// The `hyper` client
    client: Client<HttpsConnector<HttpConnector>>,
}

impl Spawner {
    /// Create a new `EchoRuntime`
    pub fn new() -> crate::error::Result<Self> {
        let https = HttpsConnector::new(4)?;
        let client = Client::builder().build::<_, Body>(https);
        let rt = Runtime::new()?;

        Ok(Self { rt, client })
    }

    /// Spawn an `Echo Event` on the inner `tokio` runtime
    pub fn spawn(&self, payload: &Payload) -> crate::error::Result<()> {
        // Clone to move into async closure
        let events_clone = payload.events.clone();
        let client = self.client.clone();
        let logger = payload.logger.clone();

        // Setup some other pre-reqs
        let uri = payload.url.as_str().to_string();
        let json = serde_json::to_string(&events_clone)?;

        let _ = self.rt.spawn(async {
            let _res = run_impl(client, logger, uri, json).await;
        });

        Ok(())
    }
}

// A simple type alias so as to DRY.
type FutResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

lazy_static! {
    static ref USER_AGENT: String =
        format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

async fn run_impl(
    client: Client<HttpsConnector<HttpConnector>>,
    logger: Option<Logger>,
    url: String,
    json: String,
) -> FutResult<()> {
    let length = json.as_bytes().len();

    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header("User-Agent", (*USER_AGENT).clone())
        .header("Content-Type", "application/json")
        .header("Content-Length", length)
        .body(Body::from(json))?;

    let resp = client.request(req).await?;

    if resp.status().is_success() {
        try_trace!(logger, "Successfully sent payload to echo");
        Ok(())
    } else {
        let err_type = if resp.status().is_client_error() {
            "Client"
        } else if resp.status().is_server_error() {
            "Server"
        } else {
            "Unknown"
        };

        try_error!(
            logger,
            "{} error sending Echo Payload: {}",
            err_type,
            resp.status()
        );
        let mut body = resp.into_body();
        let mut buffer = vec![];
        while let Some(next) = body.next().await {
            let chunk = next?;
            buffer.write_all(&chunk)?;
        }
        try_error!(logger, "{}", String::from_utf8_lossy(&buffer));
        Err(ErrKind::Run.into())
    }
}

/// The Echo messages urls
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum CollectorUrl {
    /// The stage url (https://echocollector-stage.kroger.com/echo/messages)
    Stage,
    /// The prod url (https://echocollector.kroger.com/echo/messages)
    Prod,
}

impl Default for CollectorUrl {
    fn default() -> Self {
        CollectorUrl::Stage
    }
}

impl CollectorUrl {
    /// Convert the enum to a str
    pub fn as_str(self) -> &'static str {
        match self {
            CollectorUrl::Stage => "https://echocollector-stage.kroger.com/echo/messages",
            CollectorUrl::Prod => "https://echocollector.kroger.com/echo/messages",
        }
    }
}

/// The payload for sending a batch of Echo `Event`s
#[derive(Clone, Debug, Default, Setters)]
pub struct Payload {
    /// The collector url to use
    #[set = "pub"]
    url: CollectorUrl,
    /// The batch of events to send
    #[set = "pub"]
    events: Vec<Event>,
    /// An optional `slog` logger
    #[set = "pub"]
    logger: Option<Logger>,
    /// An error count for retries, this is not serialized.
    error_count: usize,
    /// The retry count if an error occurred sending the batch
    retry_count: usize,
}

/// An Echo Event
#[derive(Clone, Debug, Default, PartialEq, Serialize, Setters)]
pub struct Event {
    /// The routing_key is what identifies the message with an application. It will become the ElasticSearch index.
    /// Valid characters are lowercase alpha numeric and '-'.
    /// The key should follow the format <application group>-<application name>-<environment>.
    #[serde(rename = "routingKey")]
    routing_key: String,
    /// Echo Event Type
    #[serde(rename = "type")]
    #[set = "pub"]
    event_type: EventType,
    /// A simple string message.  Most messages should be one line of information.  If you have secondary, deeper information to store, put it in the `message_detail`.
    ///
    /// This field holds the data when the tail appender or default log appender is used.
    message: String,
    /// The correlation id
    #[set = "pub"]
    #[serde(rename = "correlationId", skip_serializing_if = "Option::is_none")]
    correlation_id: Option<Uuid>,
    /// The timestamp of the event.  If unset, it will be set by the EchoClient.
    ///
    /// If producing your own messages, the format of the date should be either of:
    ///
    /// * An ISO-8601 date/time string (e.g. 2017-04-06T17:23:00-04:00)
    /// * A number representing milliseconds since epoch (e.g. 1491514054000)
    ///
    #[set = "pub"]
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<i64>,
    /// A place to store custom key/value pairs in the message, typically used when there isn't an appropriate root-level field.
    #[set = "pub"]
    #[serde(rename = "messageDetail", skip_serializing_if = "Option::is_none")]
    message_detail: Option<HashMap<String, String>>,
    /// Hostname where the message originated. If None, it will be set by the EchoClient.
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    /// Sets the version of the application that is creating this message.
    #[serde(rename = "applicationVersion", skip_serializing_if = "Option::is_none")]
    application_version: Option<String>,
    /// Sets the datacenter that the application is in, based on DCPloy environment settings.
    #[serde(rename = "dataCenter", skip_serializing_if = "Option::is_none")]
    data_center: Option<String>,
    /// The hostname of a client if this message is involving an external system calling into your system.
    #[serde(rename = "clientHostName", skip_serializing_if = "Option::is_none")]
    client_host_name: Option<String>,
    /// The hostname of a destination system if this message is involving your system calling an external system.
    #[serde(
        rename = "destinationHostName",
        skip_serializing_if = "Option::is_none"
    )]
    destination_host_name: Option<String>,
    /// The path being called on a destination system if this message is involving your system calling an external system.
    #[serde(rename = "destinationPath", skip_serializing_if = "Option::is_none")]
    destination_path: Option<String>,
    /// Sets the timestamp of millis since the epoch for the time at which this event started.
    #[serde(rename = "startTimestamp", skip_serializing_if = "Option::is_none")]
    #[set = "pub"]
    start_timestamp: Option<u64>,
    /// Sets the timestamp of millis since the epoch for the time at which this event finished.
    #[serde(rename = "finishTimestamp", skip_serializing_if = "Option::is_none")]
    #[set = "pub"]
    finish_timestamp: Option<u64>,
    /// Sets the duration (time in milliseconds) that passed during this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[set = "pub"]
    duration: Option<u64>,
    /// Sets the duration (time in milliseconds) that passed during this event.
    #[serde(rename = "durationInMs", skip_serializing_if = "Option::is_none")]
    #[set = "pub"]
    duration_in_ms: Option<u64>,
    /// The HTTP response code returned by a performance event.
    #[serde(rename = "responseCode", skip_serializing_if = "Option::is_none")]
    #[set = "pub"]
    response_code: Option<u16>,
    /// A more generic response used when a HTTP response code doesn't make sense. Typical values might be "success" or "failure".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[set = "pub"]
    response: Option<Response>,
}

impl Event {
    /// Set the routing key field
    pub fn set_routing_key<T>(&mut self, routing_key: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.routing_key = routing_key.into();
        self
    }

    /// Set the message field
    pub fn set_message<T>(&mut self, message: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.message = message.into();
        self
    }

    /// Set the host field
    pub fn set_host<T>(&mut self, host: Option<T>) -> &mut Self
    where
        T: Into<String>,
    {
        self.host = match host {
            None => None,
            Some(t) => Some(t.into()),
        };
        self
    }

    /// Set the application version field
    pub fn set_application_version<T>(&mut self, application_version: Option<T>) -> &mut Self
    where
        T: Into<String>,
    {
        self.application_version = match application_version {
            None => None,
            Some(t) => Some(t.into()),
        };
        self
    }

    /// Set the datacenter field
    pub fn set_data_center<T>(&mut self, data_center: Option<T>) -> &mut Self
    where
        T: Into<String>,
    {
        self.data_center = match data_center {
            None => None,
            Some(t) => Some(t.into()),
        };
        self
    }

    /// Set the client host name
    pub fn set_client_host_name<T>(&mut self, client_host_name: Option<T>) -> &mut Self
    where
        T: Into<String>,
    {
        self.client_host_name = match client_host_name {
            None => None,
            Some(t) => Some(t.into()),
        };
        self
    }

    /// Set the destination host name
    pub fn set_destination_host_name<T>(&mut self, destination_host_name: Option<T>) -> &mut Self
    where
        T: Into<String>,
    {
        self.destination_host_name = match destination_host_name {
            None => None,
            Some(t) => Some(t.into()),
        };
        self
    }

    /// Set the destination path
    pub fn set_destination_path<T>(&mut self, destination_path: Option<T>) -> &mut Self
    where
        T: Into<String>,
    {
        self.destination_path = match destination_path {
            None => None,
            Some(t) => Some(t.into()),
        };
        self
    }
}

/// Echo Event Type
///
/// The following types are currently recognized:
///
/// * ERROR - Any message that should be associated with a non-normal action or situation that the system processed.
/// * INFO - Any message that should be associated with a normal action or situation that the system processed.
/// * PERFORMANCE - Any message that associates speed or time taken with which any action or situation that the system processed.
/// * TRACKING - Any message that tries to correlate two (or more) events or data points that is not associated.
/// * SYSTEM - Internally used for client machine performance data (CPU utilization, JVM heap usage, ect)
///
/// Additional types may be added in the future.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventType {
    /// ERROR
    Error,
    /// INFO
    Info,
    /// PERFORMANCE
    Performance,
    /// TRACKING
    Tracking,
    /// SYSTEM
    System,
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Info
    }
}

impl Ser for EventType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            EventType::Error => serializer.serialize_str("ERROR"),
            EventType::Info => serializer.serialize_str("INFO"),
            EventType::Performance => serializer.serialize_str("PERFORMANCE"),
            EventType::Tracking => serializer.serialize_str("TRACKING"),
            EventType::System => serializer.serialize_str("SYSTEM"),
        }
    }
}

/// A more generic response used when a HTTP response code doesn't make sense. Typical values might be "success" or "failure".
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Response {
    /// Success
    Success,
    /// Failure
    Failure,
}

impl Default for Response {
    fn default() -> Self {
        Response::Success
    }
}

impl Ser for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Response::Success => serializer.serialize_str("success"),
            Response::Failure => serializer.serialize_str("failure"),
        }
    }
}

#[cfg(test)]
mod test {
    use {
        super::{Event, EventType, Payload, Response, Spawner},
        crate::error::Result,
        chrono::{offset::TimeZone, Utc},
        slog::{o, Drain, Logger},
        std::{collections::HashMap, sync::mpsc::channel, thread, time::Duration},
        uuid::Uuid,
    };

    #[test]
    fn serialize_default() -> Result<()> {
        let echo_event = Event::default();
        let result = serde_json::to_string(&echo_event)?;
        assert_eq!(result, r#"{"routingKey":"","type":"INFO","message":""}"#);
        Ok(())
    }

    #[test]
    fn with_message() -> Result<()> {
        let mut echo_event = Event::default();
        let _ = echo_event.set_message("testing");
        let result = serde_json::to_string(&echo_event)?;
        assert_eq!(
            result,
            r#"{"routingKey":"","type":"INFO","message":"testing"}"#
        );
        Ok(())
    }

    #[test]
    fn with_type() -> Result<()> {
        let mut echo_event = Event::default();
        let _ = echo_event.set_event_type(EventType::Performance);
        let result = serde_json::to_string(&echo_event)?;
        assert_eq!(
            result,
            r#"{"routingKey":"","type":"PERFORMANCE","message":""}"#
        );
        Ok(())
    }

    #[test]
    fn full() -> Result<()> {
        let mut echo_event = Event::default();
        let _ = echo_event.set_routing_key("atlas-dev-promises");
        let _ = echo_event.set_event_type(EventType::System);
        let _ = echo_event.set_message("testing");
        let _ = echo_event.set_correlation_id(Some(Uuid::parse_str(
            "35F3E1D6-D859-4AA0-8C58-2CDFE97A4710",
        )?));
        let _ = echo_event.set_timestamp(Some(
            Utc.ymd(1976, 3, 22)
                .and_hms_milli(0, 0, 1, 666)
                .timestamp_millis(),
        ));
        let mut message_detail = HashMap::new();
        let _ = message_detail.insert("a", "b");
        let _ = echo_event.set_message_detail(Some(
            message_detail
                .iter_mut()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ));
        let _ = echo_event.set_host(Some("host"));
        let _ = echo_event.set_application_version(Some("1.2.3"));
        let _ = echo_event.set_data_center(Some("cdc"));
        let _ = echo_event.set_client_host_name(Some("blah"));
        let _ = echo_event.set_destination_host_name(Some("blah1"));
        let _ = echo_event.set_destination_path(Some("yoda"));
        let _ = echo_event.set_start_timestamp(Some(1));
        let _ = echo_event.set_finish_timestamp(Some(2));
        let _ = echo_event.set_duration(Some(3));
        let _ = echo_event.set_duration_in_ms(Some(4));
        let _ = echo_event.set_response_code(Some(200));
        let _ = echo_event.set_response(Some(Response::Failure));

        let result = serde_json::to_string(&echo_event)?;
        assert_eq!(
            result,
            r#"{"routingKey":"atlas-dev-promises","type":"SYSTEM","message":"testing","correlationId":"35f3e1d6-d859-4aa0-8c58-2cdfe97a4710","timestamp":196300801666,"messageDetail":{"a":"b"},"host":"host","applicationVersion":"1.2.3","dataCenter":"cdc","clientHostName":"blah","destinationHostName":"blah1","destinationPath":"yoda","startTimestamp":1,"finishTimestamp":2,"duration":3,"durationInMs":4,"responseCode":200,"response":"failure"}"#
        );
        Ok(())
    }

    fn create_logger() -> Logger {
        let plain = slog_term::TermDecorator::new().build();
        let full = slog_term::FullFormat::new(plain).build().fuse();
        let drain = slog_async::Async::new(full).build().fuse();
        Logger::root(drain, o!())
    }

    #[test]
    fn single_thread_spawn() -> Result<()> {
        let echo_spawner = Spawner::new()?;

        let logger = create_logger();

        let mut echo_event = Event::default();
        let _ = echo_event.set_routing_key("atlas-local-promises");
        let _ = echo_event.set_event_type(EventType::System);
        let _ = echo_event.set_message("testing");
        let _ = echo_event.set_correlation_id(Some(Uuid::parse_str(
            "35F3E1D6-D859-4AA0-8C58-2CDFE97A4710",
        )?));
        let _ = echo_event.set_timestamp(Some(Utc::now().timestamp_millis()));

        let mut payload = Payload::default();
        let _ = payload.set_logger(Some(logger));
        let _ = payload.set_events(vec![echo_event]);

        assert!(echo_spawner.spawn(&payload).is_ok());

        // Sleep so the spawned future can complete
        thread::sleep(Duration::from_millis(1000));
        Ok(())
    }

    #[test]
    fn multi_thread_spawn() -> Result<()> {
        let echo_spawner = Spawner::new()?;
        let (tx, rx) = channel();
        let mut handles = vec![];
        let logger = create_logger();

        let mut echo_event = Event::default();
        let _ = echo_event.set_routing_key("atlas-local-promises");
        let _ = echo_event.set_event_type(EventType::System);
        let _ = echo_event.set_correlation_id(Some(Uuid::parse_str(
            "35F3E1D6-D859-4AA0-8C58-2CDFE97A4710",
        )?));
        let _ = echo_event.set_timestamp(Some(Utc::now().timestamp_millis()));

        for _ in 0..10 {
            let tx = tx.clone();
            handles.push(thread::spawn(move || {
                tx.send("message").expect("unable to send message");
            }));
        }

        let mut count = 0;
        for _ in 0..10 {
            let j = rx.recv().map_err(|e| format!("{}", e))?;
            assert_eq!(j, "message");
            let _ = echo_event.set_message(format!("Message: {}", count));
            let mut payload = Payload::default();
            let _ = payload.set_logger(Some(logger.clone()));
            let _ = payload.set_events(vec![echo_event.clone()]);
            let _ = echo_spawner.spawn(&payload);
            count += 1;
        }
        assert_eq!(count, 10);

        // Sleep so the spawned futures can complete
        thread::sleep(Duration::from_millis(2000));

        for handle in handles {
            let _ = handle.join();
        }
        Ok(())
    }
}
