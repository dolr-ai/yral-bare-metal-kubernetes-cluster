use consts::OFF_CHAIN_AGENT_URL;
use leptos::prelude::ServerFnError;
use reqwest::Client;
use serde_json::json;

pub async fn initiate_delete_user(
    user_id: String,
    id_token: String,
) -> Result<(), ServerFnError> {
    let client = Client::new();
    let body = json!({
        "user_id": user_id
    });

    let url = OFF_CHAIN_AGENT_URL.join("api/v1/user").unwrap();

    let response = client
        .delete(url)
        .bearer_auth(id_token)
        .json(&body)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(ServerFnError::ServerError(format!(
            "Delete user failed with status: {}",
            response.status()
        )))
    }
}
