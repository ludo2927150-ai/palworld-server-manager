//! Informations réseau pour inviter des joueurs : IP locale (sans rien envoyer) et IP publique (à la demande).

use crate::{Error, Result};
use std::{net::{IpAddr, UdpSocket}, time::Duration};

/// Adresse IP locale principale du PC (celle utilisée pour sortir vers Internet). Aucun paquet n'est envoyé :
/// connecter une socket UDP sert seulement à demander au système quelle interface serait utilisée.
pub fn lan_ip() -> Option<IpAddr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

/// N'accepte que du texte qui est réellement une adresse IP (jamais une page d'erreur HTML).
pub fn parse_ip(text: &str) -> Option<IpAddr> { text.trim().parse().ok() }

/// IP publique vue d'Internet, via api.ipify.org. Appelée uniquement sur demande de l'utilisateur.
pub async fn public_ip() -> Result<String> {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(6)).build()?;
    let body = http.get("https://api.ipify.org").send().await?.error_for_status()?.text().await?;
    parse_ip(&body).map(|ip| ip.to_string()).ok_or_else(|| Error::Other("réponse inattendue du service d'IP publique".into()))
}

#[cfg(test)]
mod tests {
    use super::parse_ip;

    #[test]
    fn only_real_ips_are_accepted() {
        assert_eq!(parse_ip(" 203.0.113.7\n").unwrap().to_string(), "203.0.113.7");
        assert!(parse_ip("<html>Too many requests</html>").is_none());
        assert!(parse_ip("").is_none());
    }
}
