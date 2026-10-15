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
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

struct MediaResponse<B> {
    status: u16,
    location: Option<String>,
    content_length: Option<u64>,
    body: B,
}

trait MediaBody {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, String>;
}

trait MediaTransport {
    type Body: MediaBody;

    async fn send(&mut self, url: Url) -> Result<MediaResponse<Self::Body>, String>;
}

struct ReqwestTransport;

struct ReqwestBody {
    response: reqwest::Response,
}

impl MediaBody for ReqwestBody {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, String> {
        self.response
            .chunk()
            .await
            .map(|chunk| chunk.map(|value| value.to_vec()))
            .map_err(|_| "O download da mídia foi interrompido.".into())
    }
}

impl MediaTransport for ReqwestTransport {
    type Body = ReqwestBody;

    async fn send(&mut self, url: Url) -> Result<MediaResponse<Self::Body>, String> {
        let client = client_for(&url).await?;
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|_| "Não foi possível baixar a mídia remota.")?;
        Ok(MediaResponse {
            status: response.status().as_u16(),
            location: response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            content_length: response.content_length(),
            body: ReqwestBody { response },
        })
    }
}

async fn download_with<T: MediaTransport>(transport: &mut T, url: &str) -> Result<Vec<u8>, String> {
    download_with_timeout(transport, url, DOWNLOAD_TIMEOUT).await
}

async fn download_with_timeout<T: MediaTransport>(
    transport: &mut T,
    url: &str,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    tokio::time::timeout(timeout, download_inner(transport, url))
        .await
        .map_err(|_| "O download da mídia remota excedeu o tempo limite.")?
}

async fn download_inner<T: MediaTransport>(
    transport: &mut T,
    url: &str,
) -> Result<Vec<u8>, String> {
    let mut current = Url::parse(url).map_err(|_| "A origem remota não é válida.")?;
    let mut redirects = 0;
    loop {
        validate_url(&current)?;
        let mut response = transport.send(current.clone()).await?;
        match process_response_head(
            &current,
            response.status,
            response.location.as_deref(),
            response.content_length,
        )? {
            ResponseAction::Redirect(next) => {
                if redirects == MAX_REDIRECTS {
                    return Err("A origem remota excedeu o limite de redirecionamentos.".into());
                }
                redirects += 1;
                current = next;
            }
            ResponseAction::Body => {
                let mut bytes = Vec::new();
                while let Some(chunk) = response.body.next_chunk().await? {
                    if !append_chunk(&mut bytes, &chunk) {
                        return Err("A mídia remota excede o limite experimental de 25 MB.".into());
                    }
                }
                return Ok(bytes);
            }
        }
    }
}

enum ResponseAction {
    Redirect(Url),
    Body,
}

fn process_response_head(
    current: &Url,
    status: u16,
    location: Option<&str>,
    content_length: Option<u64>,
) -> Result<ResponseAction, String> {
    if (300..400).contains(&status) {
        return Ok(ResponseAction::Redirect(next_redirect(
            current,
            location.ok_or("O redirecionamento remoto é inválido.")?,
        )?));
    }
    if !(200..300).contains(&status) {
        return Err("A origem remota recusou o download da mídia.".into());
    }
    if !accepts_content_length(content_length) {
        return Err("A mídia remota excede o limite experimental de 25 MB.".into());
    }
    Ok(ResponseAction::Body)
}

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
    use std::collections::VecDeque;

    struct FakeBody {
        chunks: VecDeque<Result<Vec<u8>, String>>,
        chunk_delay: Option<Duration>,
    }

    impl MediaBody for FakeBody {
        async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, String> {
            if let Some(delay) = self.chunk_delay.take() {
                tokio::time::sleep(delay).await;
            }
            match self.chunks.pop_front() {
                Some(chunk) => chunk.map(Some),
                None => Ok(None),
            }
        }
    }

    struct FakeTransport {
        responses: VecDeque<Result<MediaResponse<FakeBody>, String>>,
        requested: Vec<Url>,
        request_delay: Option<Duration>,
    }

    impl MediaTransport for FakeTransport {
        type Body = FakeBody;

        async fn send(&mut self, url: Url) -> Result<MediaResponse<Self::Body>, String> {
            self.requested.push(url);
            if let Some(delay) = self.request_delay.take() {
                tokio::time::sleep(delay).await;
            }
            self.responses
                .pop_front()
                .unwrap_or_else(|| Err("O transporte de teste ficou sem respostas.".into()))
        }
    }

    fn response(
        status: u16,
        location: Option<&str>,
        content_length: Option<u64>,
        chunks: impl IntoIterator<Item = Result<Vec<u8>, String>>,
    ) -> MediaResponse<FakeBody> {
        MediaResponse {
            status,
            location: location.map(str::to_owned),
            content_length,
            body: FakeBody {
                chunks: chunks.into_iter().collect(),
                chunk_delay: None,
            },
        }
    }

    fn transport(
        responses: impl IntoIterator<Item = Result<MediaResponse<FakeBody>, String>>,
    ) -> FakeTransport {
        FakeTransport {
            responses: responses.into_iter().collect(),
            requested: Vec::new(),
            request_delay: None,
        }
    }

    fn run_download(transport: &mut FakeTransport, url: &str) -> Result<Vec<u8>, String> {
        tauri::async_runtime::block_on(download_with(transport, url))
    }

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

    #[test]
    fn processes_status_redirects_and_real_body_rules() {
        let url = Url::parse("https://cdn.example.test/a.gif").unwrap();
        assert!(matches!(
            process_response_head(&url, 200, None, None),
            Ok(ResponseAction::Body)
        ));
        assert!(process_response_head(&url, 404, None, None).is_err());
        assert!(process_response_head(&url, 500, None, None).is_err());
        assert!(matches!(
            process_response_head(&url, 302, Some("https://media.example.test/b.gif"), None),
            Ok(ResponseAction::Redirect(_))
        ));
        assert!(process_response_head(&url, 302, Some("http://bad.example.test/a"), None).is_err());
        let mut bytes = Vec::new();
        assert!(append_chunk(&mut bytes, b"GIF89a-original"));
        assert_eq!(bytes, b"GIF89a-original");
    }

    #[test]
    fn download_with_uses_transport_and_preserves_gif_bytes() {
        let expected = b"GIF89a-original".to_vec();
        let mut transport = transport([Ok(response(
            200,
            None,
            None,
            [Ok(b"GIF89a-".to_vec()), Ok(b"original".to_vec())],
        ))]);

        let actual = run_download(&mut transport, "https://cdn.example.test/image.gif").unwrap();

        assert_eq!(actual, expected);
        assert_eq!(transport.requested.len(), 1);
        assert_eq!(
            transport.requested[0],
            Url::parse("https://cdn.example.test/image.gif").unwrap()
        );
    }

    #[test]
    fn download_with_rejects_http_error_responses() {
        for status in [404, 500] {
            let mut transport = transport([Ok(response(status, None, None, []))]);
            assert!(run_download(&mut transport, "https://cdn.example.test/image.gif").is_err());
        }
    }

    #[test]
    fn download_with_rejects_request_and_body_errors_without_partial_bytes() {
        let mut request_error = transport([Err("Falha de transporte simulada.".into())]);
        assert!(run_download(&mut request_error, "https://cdn.example.test/image.gif").is_err());

        let mut body_error = transport([Ok(response(
            200,
            None,
            None,
            [
                Ok(b"GIF89a-parcial".to_vec()),
                Err("Falha de leitura simulada.".into()),
            ],
        ))]);
        assert!(run_download(&mut body_error, "https://cdn.example.test/image.gif").is_err());
    }

    #[test]
    fn download_with_follows_https_redirect_and_records_requested_urls() {
        let mut transport = transport([
            Ok(response(
                302,
                Some("https://media.example.test/image.gif"),
                None,
                [],
            )),
            Ok(response(200, None, None, [Ok(b"GIF89a".to_vec())])),
        ]);

        assert_eq!(
            run_download(&mut transport, "https://cdn.example.test/image.gif").unwrap(),
            b"GIF89a"
        );
        assert_eq!(
            transport.requested,
            vec![
                Url::parse("https://cdn.example.test/image.gif").unwrap(),
                Url::parse("https://media.example.test/image.gif").unwrap(),
            ]
        );
    }

    #[test]
    fn download_with_allows_three_redirects_and_rejects_the_fourth() {
        let redirects = [
            Ok(response(
                302,
                Some("https://one.example.test/a.gif"),
                None,
                [],
            )),
            Ok(response(
                302,
                Some("https://two.example.test/a.gif"),
                None,
                [],
            )),
            Ok(response(
                302,
                Some("https://three.example.test/a.gif"),
                None,
                [],
            )),
            Ok(response(200, None, None, [Ok(b"GIF89a".to_vec())])),
        ];
        let mut accepted = transport(redirects);
        assert_eq!(
            run_download(&mut accepted, "https://cdn.example.test/image.gif").unwrap(),
            b"GIF89a"
        );
        assert_eq!(
            accepted.requested,
            vec![
                Url::parse("https://cdn.example.test/image.gif").unwrap(),
                Url::parse("https://one.example.test/a.gif").unwrap(),
                Url::parse("https://two.example.test/a.gif").unwrap(),
                Url::parse("https://three.example.test/a.gif").unwrap(),
            ]
        );

        let mut rejected = transport([
            Ok(response(
                302,
                Some("https://one.example.test/a.gif"),
                None,
                [],
            )),
            Ok(response(
                302,
                Some("https://two.example.test/a.gif"),
                None,
                [],
            )),
            Ok(response(
                302,
                Some("https://three.example.test/a.gif"),
                None,
                [],
            )),
            Ok(response(
                302,
                Some("https://four.example.test/a.gif"),
                None,
                [],
            )),
        ]);
        assert!(run_download(&mut rejected, "https://cdn.example.test/image.gif").is_err());
        assert_eq!(rejected.requested.len(), 4);
    }

    #[test]
    fn download_with_terminates_redirect_loops_and_rejects_forbidden_destinations() {
        let mut loop_transport = transport([
            Ok(response(
                302,
                Some("https://loop.example.test/b.gif"),
                None,
                [],
            )),
            Ok(response(
                302,
                Some("https://loop.example.test/a.gif"),
                None,
                [],
            )),
            Ok(response(
                302,
                Some("https://loop.example.test/b.gif"),
                None,
                [],
            )),
            Ok(response(
                302,
                Some("https://loop.example.test/a.gif"),
                None,
                [],
            )),
        ]);
        assert!(run_download(&mut loop_transport, "https://loop.example.test/a.gif").is_err());
        assert_eq!(loop_transport.requested.len(), 4);

        for location in ["http://media.example.test/a.gif", "https://127.0.0.1/a.gif"] {
            let mut transport = transport([Ok(response(302, Some(location), None, []))]);
            assert!(run_download(&mut transport, "https://cdn.example.test/image.gif").is_err());
            assert_eq!(transport.requested.len(), 1);
        }
    }

    #[test]
    fn download_with_enforces_effective_limit_when_content_length_is_absent_or_misleading() {
        let mut missing_length =
            transport([Ok(response(200, None, None, [Ok(b"GIF89a".to_vec())]))]);
        assert_eq!(
            run_download(&mut missing_length, "https://cdn.example.test/image.gif").unwrap(),
            b"GIF89a"
        );

        let mut misleading_length = transport([Ok(response(
            200,
            None,
            Some(1),
            [Ok(vec![0; MAX_BYTES + 1])],
        ))]);
        assert!(
            run_download(&mut misleading_length, "https://cdn.example.test/image.gif").is_err()
        );
    }

    #[test]
    fn download_with_timeout_limits_the_entire_operation() {
        let mut delayed_response = response(200, None, None, [Ok(b"GIF89a".to_vec())]);
        delayed_response.body.chunk_delay = Some(Duration::from_millis(25));
        let mut transport = transport([Ok(delayed_response)]);

        let result = tauri::async_runtime::block_on(download_with_timeout(
            &mut transport,
            "https://cdn.example.test/image.gif",
            Duration::from_millis(1),
        ));

        assert!(result.is_err());
        assert_eq!(transport.requested.len(), 1);
    }
}

pub async fn download(url: &str) -> Result<Vec<u8>, String> {
    download_with(&mut ReqwestTransport, url).await
}
