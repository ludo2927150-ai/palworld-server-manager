//! Ouverture du port de jeu (UDP) sur la box via UPnP. Uniquement le port du jeu : jamais l'API REST ni l'accès mobile.
//! Désactivé par défaut (`settings.upnp.enabled`) car cela expose le serveur de jeu à Internet.

use crate::{Error, Result};
use igd_next::{aio::tokio::search_gateway, PortMappingProtocol, SearchOptions};
use std::net::{IpAddr, SocketAddr};

const LEASE_SECS: u32 = 3600;
const DESCRIPTION: &str = "Palworld Server Manager";

/// Port de jeu : `PublicPort` du serveur si défini, sinon 8211.
pub fn game_port(public_port: Option<&str>) -> u16 { public_port.and_then(|p| p.trim().parse().ok()).filter(|p| *p > 1023).unwrap_or(8211) }

async fn gateway() -> Result<igd_next::aio::Gateway<igd_next::aio::tokio::Tokio>> {
    tokio::time::timeout(std::time::Duration::from_secs(8), search_gateway(SearchOptions::default())).await
        .map_err(|_| Error::Other("aucune box UPnP trouvée (délai dépassé) : UPnP est peut-être désactivé sur la box".into()))?
        .map_err(|e| Error::Other(format!("aucune box UPnP trouvée : {e}")))
}

/// Ouvre (ou renouvelle) la redirection UDP `port` → ce PC. Bail d'une heure : à renouveler tant que le serveur tourne.
/// Renvoie l'adresse IP publique vue par la box.
pub async fn open(port: u16, local: IpAddr) -> Result<String> {
    let gw = gateway().await?;
    gw.add_port(PortMappingProtocol::UDP, port, SocketAddr::new(local, port), LEASE_SECS, DESCRIPTION).await
        .map_err(|e| Error::Other(format!("la box a refusé la redirection UDP {port} : {e}")))?;
    Ok(gw.get_external_ip().await.map(|ip| ip.to_string()).unwrap_or_default())
}

/// Ferme la redirection (au désactivage de l'option ou à la fermeture de l'application).
pub async fn close(port: u16) -> Result<()> {
    let gw = gateway().await?;
    gw.remove_port(PortMappingProtocol::UDP, port).await.map_err(|e| Error::Other(format!("fermeture du port {port} : {e}")))
}

#[cfg(test)]
mod tests {
    #[test]
    fn game_port_defaults_and_rejects_privileged_ports() {
        assert_eq!(super::game_port(None), 8211);
        assert_eq!(super::game_port(Some(" 8300 ")), 8300);
        assert_eq!(super::game_port(Some("80")), 8211);
        assert_eq!(super::game_port(Some("abc")), 8211);
    }
}
