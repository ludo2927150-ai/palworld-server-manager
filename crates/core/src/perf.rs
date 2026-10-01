//! Priorité CPU et affinité (cœurs) du processus serveur. Appliqué via PowerShell sur Windows : aucun code `unsafe`,
//! et le script n'est construit qu'à partir de valeurs validées (énumération + entier), jamais de texte libre.

use crate::{settings::{PerformanceSettings, Priority}, Error, Result};

impl Priority {
    /// Nom attendu par `Process.PriorityClass` (.NET).
    pub fn dotnet_name(self) -> &'static str {
        match self { Priority::BelowNormal => "BelowNormal", Priority::Normal => "Normal", Priority::AboveNormal => "AboveNormal", Priority::High => "High" }
    }
}

/// Masque d'affinité : un bit par cœur logique (64 cœurs max). `None` (ou liste vide) = tous les cœurs de `total`.
pub fn affinity_mask(cores: Option<&[u32]>, total: usize) -> u64 {
    let total = total.clamp(1, 64);
    let all = if total == 64 { u64::MAX } else { (1u64 << total) - 1 };
    match cores {
        Some(c) if !c.is_empty() => {
            let m = c.iter().filter(|&&i| (i as usize) < total).fold(0u64, |acc, &i| acc | (1u64 << i));
            if m == 0 { all } else { m } // jamais de masque vide : le processus ne pourrait plus tourner
        }
        _ => all,
    }
}

/// Script PowerShell : applique priorité et affinité à tous les processus `PalServer*` et affiche leur nombre.
pub fn powershell_script(p: &PerformanceSettings, total_cores: usize) -> String {
    let mask = affinity_mask(p.cpu_cores.as_deref(), total_cores);
    format!(
        "$n=0; Get-Process | Where-Object {{ $_.ProcessName -like 'PalServer*' }} | ForEach-Object {{ try {{ $_.PriorityClass='{}'; $_.ProcessorAffinity=[IntPtr][int64]{}; $n++ }} catch {{ }} }}; Write-Output $n",
        p.priority.dotnet_name(), mask
    )
}

/// Applique les réglages aux processus du serveur en cours. Renvoie le nombre de processus modifiés.
#[cfg(windows)]
pub fn apply(p: &PerformanceSettings, total_cores: usize) -> Result<usize> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &powershell_script(p, total_cores)])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output().map_err(|e| Error::Other(format!("PowerShell introuvable : {e}")))?;
    if !out.status.success() { return Err(Error::Other(String::from_utf8_lossy(&out.stderr).trim().to_string())); }
    String::from_utf8_lossy(&out.stdout).trim().parse::<usize>().map_err(|_| Error::Other("réponse inattendue de PowerShell".into()))
}

#[cfg(not(windows))]
pub fn apply(_p: &PerformanceSettings, _total_cores: usize) -> Result<usize> {
    Err(Error::Other("la priorité et les cœurs ne se règlent que sous Windows".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_selects_cores_and_never_empty() {
        assert_eq!(affinity_mask(None, 8), 0b1111_1111);
        assert_eq!(affinity_mask(Some(&[]), 4), 0b1111);
        assert_eq!(affinity_mask(Some(&[0, 2, 3]), 8), 0b1101);
        assert_eq!(affinity_mask(Some(&[9, 12]), 8), 0b1111_1111); // cœurs inexistants → tous
        assert_eq!(affinity_mask(None, 64), u64::MAX);
        assert_eq!(affinity_mask(Some(&[1, 99]), 4), 0b10);
    }

    #[test]
    fn script_uses_only_validated_values() {
        let p = PerformanceSettings { priority: Priority::High, cpu_cores: Some(vec![0, 1]), ..Default::default() };
        let s = powershell_script(&p, 8);
        assert!(s.contains("PriorityClass='High'") && s.contains("[int64]3;"));
        assert!(powershell_script(&PerformanceSettings::default(), 4).contains("PriorityClass='Normal'"));
    }
}
