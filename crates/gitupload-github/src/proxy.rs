//! Corporate NTLM/HTTP proxy routing & custom Enterprise Root CA certificate manager.

use gitupload_core::errors::{GitHubError, GitHubResult};
use gitupload_core::AirGapGuard;
use reqwest::{Certificate, Client, NoProxy, Proxy};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Corporate outbound proxy and self-signed enterprise CA configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorporateProxyConfig {
    /// Forward proxy URL (`http://proxy.corp.internal:8080`). Empty disables proxy.
    pub proxy_url: String,
    /// Optional path to a PEM-encoded Enterprise Root CA certificate bundle.
    pub custom_ca_cert_path: Option<String>,
    /// Comma-separated hosts bypassing the corporate proxy (`localhost,*.internal`).
    #[serde(default)]
    pub bypass_hosts: Vec<String>,
}

impl CorporateProxyConfig {
    /// Builds a hardened `reqwest::Client` configured with enterprise proxy and optional Root CA cert.
    pub fn build_http_client(&self) -> GitHubResult<Client> {
        let mut builder = Client::builder()
            .user_agent("GitUpload-Desktop/2.5")
            .timeout(Duration::from_secs(30));

        if !self.proxy_url.trim().is_empty() {
            AirGapGuard::assert_network_allowed(self.proxy_url.trim())
                .map_err(|e| GitHubError::Network(e.to_string()))?;

            let mut proxy = Proxy::all(self.proxy_url.trim())
                .map_err(|e| GitHubError::Network(format!("Invalid corporate proxy URL: {e}")))?;

            if !self.bypass_hosts.is_empty() {
                let joined = self.bypass_hosts.join(",");
                proxy = proxy.no_proxy(NoProxy::from_string(&joined));
            }
            builder = builder.proxy(proxy);
        }

        if let Some(ca_path) = self
            .custom_ca_cert_path
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            let pem_bytes = std::fs::read(ca_path).map_err(|e| {
                GitHubError::Network(format!("Failed reading custom CA PEM '{ca_path}': {e}"))
            })?;
            let pem_str = String::from_utf8_lossy(&pem_bytes);
            if !pem_str.contains("-----BEGIN CERTIFICATE-----") {
                return Err(GitHubError::Network(format!(
                    "Invalid X.509 PEM certificate '{ca_path}': missing CERTIFICATE block"
                )));
            }
            let cert = Certificate::from_pem(&pem_bytes).map_err(|e| {
                GitHubError::Network(format!("Invalid X.509 PEM certificate '{ca_path}': {e}"))
            })?;
            builder = builder.add_root_certificate(cert);
        }

        builder
            .build()
            .map_err(|e| GitHubError::Network(format!("Failed constructing HTTP client: {e}")))
    }
}
