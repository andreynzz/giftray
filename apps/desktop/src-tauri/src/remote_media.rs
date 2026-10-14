// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use reqwest::{redirect::Policy, Client, Url};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

const MAX_BYTES: usize = 25 * 1024 * 1024;
const MAX_REDIRECTS: usize = 3;

fn public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => {
            !value.is_private()
                && !value.is_loopback()
                && !value.is_link_local()
                && !value.is_unspecified()
                && !value.is_multicast()
                && !value.is_broadcast()
                && !value.is_documentation()
        }
        IpAddr::V6(value) => {
            !value.is_loopback()
                && !value.is_unspecified()
                && !value.is_multicast()
                && !value.is_unique_local()
                && !value.is_unicast_link_local()
        }
    }
}

fn validate_url(url: &Url) -> Result<&str, String> {
    if url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("A origem remota não atende aos requisitos de segurança.".into());
    }
    let host = url.host_str().ok_or("A origem remota não é válida.")?;
    if host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.parse::<IpAddr>().is_ok()
    {
        return Err("A origem remota não atende aos requisitos de segurança.".into());
    }
    Ok(host)
}

async fn client_for(url: &Url) -> Result<Client, String> {
    let host = validate_url(url)?;
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, 443))
        .await
        .map_err(|_| "Não foi possível resolver a origem remota.")?
        .filter(|address| public_address(address.ip()))
        .collect();
    if addresses.is_empty() {
        return Err("A origem remota não atende aos requisitos de segurança.".into());
    }
    // resolve_to_addrs binds reqwest to the exact validated addresses, preventing a second DNS resolution/rebinding.
    Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .resolve_to_addrs(host, &addresses)
        .build()
        .map_err(|_| "Não foi possível preparar o download seguro.".into())
}

pub async fn download(url: &str) -> Result<Vec<u8>, String> {
    let mut current = Url::parse(url).map_err(|_| "A origem remota não é válida.")?;
    for _ in 0..=MAX_REDIRECTS {
        let client = client_for(&current).await?;
        let response = client
            .get(current.clone())
            .send()
            .await
            .map_err(|_| "Não foi possível baixar a mídia remota.")?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or("O redirecionamento remoto é inválido.")?;
            current = current
                .join(location)
                .map_err(|_| "O redirecionamento remoto é inválido.")?;
            continue;
        }
        if !response.status().is_success() {
            return Err("A origem remota recusou o download da mídia.".into());
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_BYTES as u64)
        {
            return Err("A mídia remota excede o limite experimental de 25 MB.".into());
        }
        let mut body = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = body
            .chunk()
            .await
            .map_err(|_| "O download da mídia foi interrompido.")?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_BYTES {
                return Err("A mídia remota excede o limite experimental de 25 MB.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        return Ok(bytes);
    }
    Err("A origem remota excedeu o limite de redirecionamentos.".into())
}
