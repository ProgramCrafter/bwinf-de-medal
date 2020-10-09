/*  medal                                                                                                            *\
 *  Copyright (C) 2020  Bundesweite Informatikwettbewerbe                                                            *
 *                                                                                                                   *
 *  This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero        *
 *  General Public License as published  by the Free Software Foundation, either version 3 of the License, or (at    *
 *  your option) any later version.                                                                                  *
 *                                                                                                                   *
 *  This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the       *
 *  implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public      *
 *  License for more details.                                                                                        *
 *                                                                                                                   *
 *  You should have received a copy of the GNU Affero General Public License along with this program.  If not, see   *
\*  <http://www.gnu.org/licenses/>.                                                                                  */

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct OauthProvider {
    pub provider_id: String,
    pub medal_oauth_type: String,
    pub url: String,
    pub client_id: String,
    pub client_secret: String,
    pub access_token_url: String,
    pub user_data_url: String,
    pub school_data_url: Option<String>,
    pub school_data_secret: Option<String>,
    pub login_link_text: String,
}
