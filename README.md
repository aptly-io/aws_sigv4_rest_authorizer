# Dabbling in rust.

This code piece generates a AWS sigv4 based on a cognito identity.
The cognito authentication uses a username and password.
From there a temporary AIM credential is generated.
That credential's secret is used to sign the REST request.

All settings are stored in the demo_settings.json as follows:

```json
{
    "region": "eu-west-1",
    "client_id": "xxxxxxxxxxxxxxxxxxxxxxxxxx",
    "user_pool": "eu-west-1_yyyyyyyyy",
    "ident_pool": "eu-west-1:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
    
    "login": "email@host.com",
    "password": "een straf paswoord",

    "url": "https://rest-api.com/resource-group/resource?status=ENABLED&limit=99"
}
```

These crates are used:

```bash
cargo add tokio
cargo add serde_json
cargo add serde --features derive
cargo add reqwest
cargo add aws-credential-types --features hardcoded-credentials
cargo add aws-sdk-cognitoidentity
cargo add aws-sdk-cognitoidentityprovider
cargo add aws-sigv4
cargo add aws-config
cargo add aws-smithy-runtime-api
cargo add aws-types
cargo add tokio --features rt-multi-thread,macros

cargo run
```