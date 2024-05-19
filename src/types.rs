use hyper::client::connect::HttpConnector;
use hyper::Client;
use hyper_tls::HttpsConnector;

pub type HttpsClient = Client<HttpsConnector<HttpConnector>>;

pub mod fintoc;
pub mod lunchmoney;
pub mod venmo;
