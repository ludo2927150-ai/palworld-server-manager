//! Lecture/écriture de `PalWorldSettings.ini`.
//!
//! Format : une seule ligne `OptionSettings=(Clé=Valeur,Clé="Texte",...)` sous la section
//! `[/Script/Pal.PalGameWorldSettings]`. Les valeurs sont conservées en `String` (guillemets retirés
//! à la lecture) ; le typage est porté par le schéma côté frontend. L'ordre des clés est préservé.

use crate::{Error, Result};

pub const SECTION: &str = "[/Script/Pal.PalGameWorldSettings]";
const KEY: &str = "OptionSettings=";

/// Une option : `quoted` mémorise si la valeur était entre guillemets, pour un aller-retour fidèle
/// (les textes libres sont quotés, les nombres/booléens/énumérations ne le sont pas).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Opt {
    pub key: String,
    pub value: String,
    pub quoted: bool,
}

pub type Options = Vec<Opt>;

pub fn parse(content: &str) -> Result<Options> {
    let line = content
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(KEY))
        .ok_or_else(|| Error::Ini("ligne OptionSettings introuvable".into()))?;
    let inner = line[KEY.len()..]
        .trim()
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| Error::Ini("parenthèses manquantes".into()))?;

    let mut out = Vec::new();
    let (mut buf, mut in_str, mut depth) = (String::new(), false, 0i32);
    let flush = |buf: &mut String, out: &mut Options| -> Result<()> {
        if buf.trim().is_empty() { buf.clear(); return Ok(()); }
        let (k, v) = buf.split_once('=').ok_or_else(|| Error::Ini(format!("paire invalide : {buf}")))?;
        let v = v.trim();
        let unq = v.strip_prefix('"').and_then(|s| s.strip_suffix('"'));
        out.push(Opt { key: k.trim().to_string(), value: unq.unwrap_or(v).to_string(), quoted: unq.is_some() });
        buf.clear();
        Ok(())
    };
    for c in inner.chars() {
        match c {
            '"' => { in_str = !in_str; buf.push(c); }
            '(' if !in_str => { depth += 1; buf.push(c); }
            ')' if !in_str => { depth -= 1; buf.push(c); }
            ',' if !in_str && depth == 0 => flush(&mut buf, &mut out)?,
            _ => buf.push(c),
        }
    }
    flush(&mut buf, &mut out)?;
    Ok(out)
}

/// Modifie ou ajoute une option (`quoted` n'est utilisé qu'à l'ajout).
pub fn set(options: &mut Options, key: &str, value: &str, quoted: bool) {
    match options.iter_mut().find(|o| o.key == key) {
        Some(o) => o.value = value.to_string(),
        None => options.push(Opt { key: key.into(), value: value.into(), quoted }),
    }
}

pub fn get<'a>(options: &'a Options, key: &str) -> Option<&'a str> {
    options.iter().find(|o| o.key == key).map(|o| o.value.as_str())
}

/// Refuse ce qui casserait le fichier : clé vide ou étrange, saut de ligne, guillemet dans une valeur entre guillemets
/// (un nom de serveur contenant `"` rendrait toute la ligne `OptionSettings` illisible et le serveur ne démarrerait plus).
pub fn validate(options: &Options) -> Result<()> {
    for o in options {
        if o.key.is_empty() || !o.key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(crate::Error::Other(format!("nom d'option invalide : « {} »", o.key)));
        }
        if o.value.contains(['\r', '\n']) { return Err(crate::Error::Other(format!("{} : la valeur ne peut pas contenir de saut de ligne", o.key))); }
        if o.quoted && o.value.contains('"') { return Err(crate::Error::Other(format!("{} : la valeur ne peut pas contenir de guillemet (\")", o.key))); }
    }
    Ok(())
}

pub fn serialize(options: &Options) -> String {
    let body = options
        .iter()
        .map(|o| if o.quoted { format!("{}=\"{}\"", o.key, o.value) } else { format!("{}={}", o.key, o.value) })
        .collect::<Vec<_>>()
        .join(",");
    format!("{SECTION}\r\n{KEY}({body})\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[/Script/Pal.PalGameWorldSettings]\nOptionSettings=(Difficulty=None,ExpRate=1.500000,ServerName=\"Mon, serveur\",bIsPvP=False,DeathPenalty=All,CrossplayPlatforms=(Steam,Xbox))\n";

    #[test]
    fn parses_quotes_commas_and_lists() {
        let o = parse(SAMPLE).unwrap();
        assert_eq!(o[1].value, "1.500000");
        assert!(!o[1].quoted);
        assert_eq!((o[2].value.as_str(), o[2].quoted), ("Mon, serveur", true));
        assert_eq!(o[5].value, "(Steam,Xbox)");
    }

    #[test]
    fn roundtrip_is_stable() {
        let o = parse(SAMPLE).unwrap();
        let out = serialize(&o);
        assert!(out.contains("ServerName=\"Mon, serveur\",bIsPvP=False"));
        assert_eq!(parse(&out).unwrap(), o);
    }

    #[test]
    fn set_updates_or_appends() {
        let mut o = parse(SAMPLE).unwrap();
        set(&mut o, "ExpRate", "2.000000", false);
        set(&mut o, "RESTAPIEnabled", "True", false);
        assert_eq!(get(&o, "ExpRate"), Some("2.000000"));
        assert_eq!(get(&o, "RESTAPIEnabled"), Some("True"));
        assert_eq!(get(&o, "Nope"), None);
    }

    #[test]
    fn missing_line_is_error() {
        assert!(parse("[x]\n").is_err());
    }

    #[test]
    fn validate_rejects_what_would_corrupt_the_file() {
        let ok = |k: &str, v: &str, q: bool| vec![Opt { key: k.into(), value: v.into(), quoted: q }];
        assert!(validate(&ok("ServerName", "Mon serveur, le meilleur", true)).is_ok());
        assert!(validate(&ok("CrossplayPlatforms", "(Steam,Xbox)", false)).is_ok());
        assert!(validate(&ok("ServerName", "Mon \"serveur\"", true)).is_err());
        assert!(validate(&ok("ServerName", "a\nb", true)).is_err());
        assert!(validate(&ok("Bad Key", "x", false)).is_err());
        assert!(validate(&ok("", "x", false)).is_err());
    }
}
