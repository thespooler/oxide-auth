use super::{Authorizer, ClientParameter, QueryMap, Registrar, RegistrarError};
use super::{Negotiated, NegotiationParameter};
use super::{Issuer, IssuedToken, Request};
use std::borrow::Cow;
use url::Url;
use chrono::Utc;

pub fn decode_query<'u>(kvpairs: QueryMap<'u>) -> Result<ClientParameter<'u>, String> {

    match kvpairs.get("response_type").map(|s| *s == "code") {
        None => return Err("Response type needs to be set".to_string()),
        Some(false) => return Err("Invalid response type".to_string()),
        Some(true) => ()
    }
    let client_id = match kvpairs.get("client_id") {
        None => return Err("client_id needs to be set".to_string()),
        Some(s) => s.clone()
    };
    let redirect_url = match kvpairs.get("redirect_url").map(|st| Url::parse(st)) {
        Some(Err(_)) => return Err("Invalid url".to_string()),
        val => val.map(|v| Cow::Owned(v.unwrap()))
    };
    let state = kvpairs.get("state").map(|v| v.clone());
    Ok(ClientParameter {
        client_id: client_id,
        scope: kvpairs.get("scope").map(|v| v.clone()),
        redirect_url: redirect_url,
        state: state
    })
}

pub struct CodeRef<'a> {
    registrar: &'a Registrar,
    authorizer: &'a mut Authorizer,
}

pub enum CodeError {
    Ignore /* Ignore the request entirely */,
    Redirect(Url) /* Redirect to the given url */,
}

impl<'u> CodeRef<'u> {
    pub fn negotiate<'a>(&self, client_id: Cow<'a, str>, scope: Option<Cow<'a, str>>, redirect_url: Option<Cow<'a, Url>>)
    -> Result<Negotiated<'a>, CodeError> {
        let result = match self.registrar.negotiate(NegotiationParameter{client_id, scope, redirect_url}) {
            Err(RegistrarError::Unregistered) => return Err(CodeError::Ignore),
            Err(RegistrarError::MismatchedRedirect) => return Err(CodeError::Ignore),
            Err(RegistrarError::Error(err)) => return Err(CodeError::Redirect(unimplemented!())),
            Ok(negotiated) => negotiated,
        };
        Ok(result)
    }

    pub fn authorize<'a>(&'a mut self, owner_id: Cow<'a, str>, negotiated: Negotiated<'a>, state: Option<Cow<'a, str>>)
     -> Result<Url, CodeError> {
        let grant = self.authorizer.authorize(Request{
            owner_id: &owner_id,
            client_id: &negotiated.client_id,
            redirect_url: &negotiated.redirect_url,
            scope: &negotiated.scope});
        let mut url = negotiated.redirect_url;
        url.query_pairs_mut()
            .append_pair("code", grant.as_str())
            .extend_pairs(state.map(|v| ("state", v)))
            .finish();
        Ok(url)
    }

    pub fn with<'a>(registrar: &'a Registrar, t: &'a mut Authorizer) -> CodeRef<'a> {
        CodeRef { registrar, authorizer: t }
    }
}

pub struct IssuerRef<'a> {
    authorizer: &'a mut Authorizer,
    issuer: &'a mut Issuer,
}

impl<'u> IssuerRef<'u> {
    pub fn use_code<'a>(&'a mut self, code: String, expected_client: Cow<'a, str>, expected_url: Cow<'a, str>)
    -> Result<IssuedToken, Cow<'static, str>> {
        let saved_params = match self.authorizer.recover_parameters(code.as_ref()) {
            None => return Err("Inactive code".into()),
            Some(v) => v,
        };

        if saved_params.client_id != expected_client || expected_url != saved_params.redirect_url.as_str() {
            return Err("Invalid code".into())
        }

        if saved_params.until.as_ref() < &Utc::now() {
            return Err("Code no longer valid".into())
        }

        let token = self.issuer.issue(Request{
            client_id: &saved_params.client_id,
            owner_id: &saved_params.owner_id,
            redirect_url: &saved_params.redirect_url,
            scope: &saved_params.scope,
        });
        Ok(token)
    }

    pub fn with<'a>(t: &'a mut Authorizer, i: &'a mut Issuer) -> IssuerRef<'a> {
        IssuerRef { authorizer: t, issuer: i }
    }
}
