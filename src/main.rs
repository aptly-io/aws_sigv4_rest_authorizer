use std::str::FromStr;
use std::time::SystemTime;

use aws_credential_types::Credentials;

use aws_sdk_cognitoidentity::operation::get_credentials_for_identity::GetCredentialsForIdentityError;
use aws_sdk_cognitoidentity::operation::get_id::GetIdError;
use aws_sdk_cognitoidentityprovider::operation::initiate_auth::InitiateAuthError;

use aws_sigv4::http_request::{sign, SignableBody, SignableRequest, SigningSettings};

use aws_sigv4::sign::v4;
use aws_smithy_runtime_api::client::result::SdkError;
use aws_smithy_runtime_api::http::Response;

use aws_smithy_runtime_api::client::identity::Identity;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{self, Client};

use aws_sdk_cognitoidentity;
use aws_sdk_cognitoidentityprovider;
use aws_types::region::Region;

use aws_sigv4::http_request::SigningError;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct DemoSettings {
    region: String,
    client_id: String,
    user_pool: String,
    ident_pool: String,

    login: String,
    password: String,

    url: String,
}

#[derive(Debug)]
enum Sigv4Error {
    InitiateAuth(SdkError<InitiateAuthError, Response>),
    EmptyIdToken,
    GetId(SdkError<GetIdError, Response>),
    EmptyIdentity,
    GetCredentialsForIdentity(SdkError<GetCredentialsForIdentityError, Response>),
    EmptyCredentials,
    Signing(SigningError),
}

impl std::error::Error for Sigv4Error {}
impl std::fmt::Display for Sigv4Error {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
impl From<SdkError<InitiateAuthError, Response>> for Sigv4Error {
    fn from(err: SdkError<InitiateAuthError, Response>) -> Sigv4Error {
        Sigv4Error::InitiateAuth(err)
    }
}
impl From<SdkError<GetIdError, Response>> for Sigv4Error {
    fn from(err: SdkError<GetIdError, Response>) -> Sigv4Error {
        Sigv4Error::GetId(err)
    }
}
impl From<SdkError<GetCredentialsForIdentityError, Response>> for Sigv4Error {
    fn from(err: SdkError<GetCredentialsForIdentityError, Response>) -> Sigv4Error {
        Sigv4Error::GetCredentialsForIdentity(err)
    }
}
impl From<SigningError> for Sigv4Error {
    fn from(err: SigningError) -> Sigv4Error {
        Sigv4Error::Signing(err)
    }
}

async fn get_id_token(
    config: &aws_config::SdkConfig,
    client_id: &str,
    login: &str,
    password: &str,
) -> Result<String, Sigv4Error> {
    let result = aws_sdk_cognitoidentityprovider::Client::new(&config)
        .initiate_auth()
        .auth_flow(aws_sdk_cognitoidentityprovider::types::AuthFlowType::UserPasswordAuth)
        .client_id(client_id)
        .auth_parameters("USERNAME", login)
        .auth_parameters("PASSWORD", password)
        .send()
        .await?;
    Ok(result
        .authentication_result()
        .ok_or(Sigv4Error::EmptyIdToken)?
        .id_token()
        .ok_or(Sigv4Error::EmptyIdToken)?
        .to_string())
}

async fn get_credentials(
    config: &aws_config::SdkConfig,
    region: &str,
    user_pool: &str,
    ident_pool: &str,
    id_token: &str,
) -> Result<Credentials, Sigv4Error> {
    let login_key = format!("cognito-idp.{}.amazonaws.com/{}", region, user_pool);

    let client = aws_sdk_cognitoidentity::Client::new(&config);

    let identity = client
        .get_id()
        .identity_pool_id(ident_pool)
        .logins(login_key.clone(), id_token)
        .send()
        .await?
        .identity_id()
        .ok_or(Sigv4Error::EmptyIdentity)?
        .to_string();

    let result = client
        .get_credentials_for_identity()
        .identity_id(identity)
        .logins(login_key, id_token)
        .send()
        .await?
        .credentials()
        .ok_or(Sigv4Error::EmptyCredentials)?
        .clone();

    Ok(Credentials::from_keys(
        result.access_key_id().unwrap(),
        result.secret_key().unwrap(),
        result.session_token.clone(),
    ))
}

fn get_signing_headers(
    identity: &Identity,
    method: &str,
    url: &str,
) -> Result<HeaderMap, Sigv4Error> {
    let signable_request =
        SignableRequest::new(method, url, std::iter::empty(), SignableBody::Bytes(&[]))?;

    let signing_params = v4::SigningParams::builder()
        .identity(identity)
        .settings(SigningSettings::default())
        .region("eu-west-1")
        .name("execute-api")
        .time(SystemTime::now())
        .build()
        .unwrap()
        .into();

    let signing_instructions = sign(signable_request, &signing_params)?.into_parts().0;

    let mut dest_headers = HeaderMap::new();
    for (key, val) in signing_instructions.headers() {
        let name = HeaderName::from_str(key).unwrap();
        let value = HeaderValue::from_str(val).unwrap();
        dest_headers.insert(name, value);
    }

    Ok(dest_headers)
}

#[tokio::main]
async fn main() {
    let settings: DemoSettings = serde_json::from_str(
        &std::fs::read_to_string("demo_settings.json").expect("Failed reading demo_settings.json"),
    )
    .expect("Failed converting json to DemoSettings");

    let config = aws_config::defaults(aws_config::BehaviorVersion::v2023_11_09())
        .region(Region::new(settings.region.clone()))
        .no_credentials()
        .load()
        .await;

    let id_token = get_id_token(
        &config,
        &settings.client_id,
        &settings.login,
        &settings.password,
    )
    .await
    .expect("get_id_token() failed");

    let identity = get_credentials(
        &config,
        &settings.region,
        &settings.user_pool,
        &settings.ident_pool,
        &id_token,
    )
    .await
    .expect("get_credentials() failed")
    .into();

    let signing_headers =
        get_signing_headers(&identity, "GET", &settings.url).expect("get_signing_headers() failed");

    let response = Client::new()
        .get(settings.url)
        .headers(signing_headers)
        .send()
        .await
        .expect("HTTP request failed");

    println!("response.text: {}", response.text().await.unwrap());
}
