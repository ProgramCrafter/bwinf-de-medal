#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct OauthProvider {
    pub provider_id: String,
    pub url: String,
    pub client_id: String,
    pub client_secret: String,
    pub access_token_url: String,
    pub user_data_url: String,
    pub login_link_text: String,
}
