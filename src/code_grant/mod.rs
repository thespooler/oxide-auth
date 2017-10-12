use chrono::DateTime;
use chrono::Utc;

use iron::modifiers::Redirect;
use iron::IronResult;
use iron::Response;
use iron::Url;
use iron::Request as IRequest;

use std;

pub struct NegotiationParams<'a> {
    pub client_id: &'a str,
    pub scope: Option<&'a str>,
    pub redirect_url: Option<&'a Url>
}

pub struct Negotiated {
    pub redirect_url: Url,
    pub scope: String
}

pub struct Request<'a> {
    pub owner_id: &'a str,
    pub client_id: &'a str,
    pub redirect_url: &'a Url,
    pub scope: &'a str,
}

pub struct Grant<'a> {
    pub owner_id: &'a str,
    pub client_id: &'a str,
    pub redirect_url: &'a Url,
    pub scope: &'a str,
    pub until: &'a DateTime<Utc>
}

pub trait Authorizer {
    fn negotiate(&self, NegotiationParams) -> Result<Negotiated, String>;
    fn authorize(&mut self, Request) -> String;
    fn recover_parameters<'a>(&'a self, &'a str) -> Option<Grant<'a>>;
}

pub trait WebRequest {
    fn owner_id(&self) -> Option<String>;
}

type QueryMap<'a> = std::collections::HashMap<std::borrow::Cow<'a, str>, std::borrow::Cow<'a, str>>;

fn decode_query<'u>(query: &'u Url) -> QueryMap<'u> {
    query.as_ref().query_pairs()
        .collect::<QueryMap<'u>>()
}

impl<'a, 'b> WebRequest for IRequest<'a, 'b> {
    fn owner_id(&self) -> Option<String> {
        return Some("test".to_string());
    }
}

pub trait CodeGranter {
    fn authorizer_mut(&mut self) -> &mut Authorizer;
    fn authorizer(&self) -> &Authorizer;

    fn auth_url_encoded<'u>(&'u self, query: &'u QueryMap<'u>)
    -> Result<(String, Negotiated), String> {
        match query.get("response_type").map(|s| *s == "code") {
            None => return Err("Response type needs to be set".to_string()),
            Some(false) => return Err("Invalid response type".to_string()),
            Some(true) => ()
        }
        let client_id = match query.get("client_id") {
            None => return Err("client_id needs to be set".to_string()),
            Some(s) => s
        };
        let redirect_url = match query.get("redirect_url").map(|st| Url::parse(st)) {
            Some(Err(_)) => return Err("Invalid url".to_string()),
            val => val.map(|v| v.unwrap())
        };
        let result = self.authorizer().negotiate(NegotiationParams {
            client_id: client_id,
            scope: query.get("scope").map(|s| s.as_ref()),
            redirect_url: redirect_url.as_ref() }
        )?;
        Ok((client_id.to_string(), result))
    }

    fn authorize(&mut self, client_id: String, owner_id: String, negotiated: Negotiated, state: Option<&str>) -> Url {
        let grant = self.authorizer_mut().authorize(Request{
            owner_id: &owner_id,
            client_id: &client_id,
            redirect_url: &negotiated.redirect_url,
            scope: &negotiated.scope});
        let mut url = negotiated.redirect_url;
        url.as_mut().query_pairs_mut()
            .append_pair("code", grant.as_str())
            .extend_pairs(state.map(|v| ("state", v)))
            .finish();
        url
    }
}

pub mod iron;
pub mod authorizer;
