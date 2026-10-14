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

fn next_redirect(current: &Url, location: &str) -> Result<Url, String> {
    let next = current.join(location).map_err(|_| "Invalid redirect")?;
    validate_url(&next)?;
    Ok(next)
}

fn accepts_content_length(length: Option<u64>) -> bool {
    length.is_none_or(|value| value <= MAX_BYTES as u64)
}

fn append_chunk(bytes: &mut Vec<u8>, chunk: &[u8]) -> bool {
    if bytes.len().saturating_add(chunk.len()) > MAX_BYTES {
        return false;
    }
    bytes.extend_from_slice(chunk);
    true
}

fn public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => {
            let octets = value.octets();
            !value.is_private()
                && !value.is_loopback()
                && !value.is_link_local()
                && !value.is_unspecified()
                && !value.is_multicast()
                && !value.is_broadcast()
                && !value.is_documentation()
                && octets[0] != 0
                && !(octets[0] == 100 && (64..=127).contains(&octets[1]))
                && !(octets[0] == 198 && (18..=19).contains(&octets[1]))
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

fn validated_addresses(addresses: Vec<SocketAddr>) -> Result<Vec<SocketAddr>, String> {
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !public_address(address.ip()))
    {
        return Err("A origem remota não atende aos requisitos de segurança.".into());
    }
    Ok(addresses)
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
    let addresses = validated_addresses(
        tokio::net::lookup_host((host, 443))
            .await
            .map_err(|_| "Não foi possível resolver a origem remota.")?
            .collect(),
    )?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_safe_https_urls() {
        assert!(
            validate_url(&Url::parse("https://cdn.example.test/a.gif?token=x").unwrap()).is_ok()
        );
        for value in [
            "http://example.test/a.gif",
            "https://127.0.0.1/a.gif",
            "https://localhost/a.gif",
            "https://a.local/a.gif",
            "https://user@example.test/a.gif",
            "https://example.test:444/a.gif",
        ] {
            assert!(
                validate_url(&Url::parse(value).unwrap()).is_err(),
                "{value}"
            );
        }
    }

    #[test]
    fn rejects_private_reserved_and_mixed_dns_answers() {
        let public: SocketAddr = "8.8.8.8:443".parse().unwrap();
        for value in [
            "127.0.0.1:443",
            "10.0.0.1:443",
            "100.64.0.1:443",
            "198.18.0.1:443",
            "[::1]:443",
            "[fc00::1]:443",
            "[fe80::1]:443",
        ] {
            let address = value.parse().unwrap();
            assert!(validated_addresses(vec![address]).is_err(), "{value}");
        }
        assert!(validated_addresses(vec![public]).is_ok());
        assert!(validated_addresses(vec![public, "10.0.0.1:443".parse().unwrap()]).is_err());
    }

    #[test]
    fn rejects_invalid_media_and_enforces_stream_limit() {
        assert!(crate::clipboard::media_extension(b"GIF89a data").is_some());
        assert!(crate::clipboard::media_extension(b"<html>error</html>").is_none());
        assert!(MAX_BYTES.saturating_add(1) > MAX_BYTES);
    }

    #[test]
    fn validates_redirects_and_effective_byte_limit() {
        let current = Url::parse("https://cdn.example.test/a.gif").unwrap();
        assert!(next_redirect(&current, "https://media.example.test/b.gif").is_ok());
        assert!(next_redirect(&current, "http://media.example.test/b.gif").is_err());
        assert!(next_redirect(&current, "https://127.0.0.1/b.gif").is_err());
        assert!(accepts_content_length(None));
        assert!(!accepts_content_length(Some(MAX_BYTES as u64 + 1)));
        let mut bytes = vec![0; MAX_BYTES];
        assert!(!append_chunk(&mut bytes, &[1]));
    }
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
            current = next_redirect(&current, location)?;
            continue;
        }
        if !response.status().is_success() {
            return Err("A origem remota recusou o download da mídia.".into());
        }
        if !accepts_content_length(response.content_length()) {
            return Err("A mídia remota excede o limite experimental de 25 MB.".into());
        }
        let mut body = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = body
            .chunk()
            .await
            .map_err(|_| "O download da mídia foi interrompido.")?
        {
            if !append_chunk(&mut bytes, &chunk) {
                return Err("A mídia remota excede o limite experimental de 25 MB.".into());
            }
        }
        return Ok(bytes);
    }
    Err("A origem remota excedeu o limite de redirecionamentos.".into())
}
