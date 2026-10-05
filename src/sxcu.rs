//! ShareX custom uploaders (`.sxcu`) — upload to a host the user brings.
//!
//! Not a format of our own: `.sxcu` is what image hosts and self-hosted
//! servers (Zipline, XBackBone, Chevereto, …) already publish for ShareX, so a
//! user can point Snipdeck anywhere ShareX goes without anyone writing a config
//! for us. The subset kept here is what an image upload needs, and a file that
//! needs more is refused when it is imported, with the reason — never accepted
//! and then failing at upload time:
//!
//! - **Request:** POST, PUT or PATCH to an http(s) URL, with query
//!   `Parameters`, `Headers`, and the image as `MultipartFormData` (in the
//!   `FileFormName` field, beside the text `Arguments`) or as a `Binary` body.
//! - **Input syntax:** `{filename}`. Anything that would ask the user or make
//!   up a value (`{prompt:…}`, `{random:…}`, …) is refused.
//! - **Output syntax:** `{response}`, `{responseurl}`, `{header:name}`,
//!   `{json:path}` and `{regex:pattern|group}`, and their ShareX 12 forms
//!   (`$json:path$`, `$regex:index,group$` into `RegexList`, …). An empty `URL`
//!   means the response body is the link, as in ShareX.
//!
//! The request is built by a pure function and the link read by another, so
//! both are tested without a network; one test runs the whole upload against
//! a server on the loopback interface.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One `.sxcu` file, as ShareX writes it. Unknown fields are ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Uploader {
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Version", default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "DestinationType", default, skip_serializing_if = "Option::is_none")]
    pub destination_type: Option<String>,
    /// ShareX 12 called it `RequestType`.
    #[serde(rename = "RequestMethod", alias = "RequestType", default, skip_serializing_if = "Option::is_none")]
    pub request_method: Option<String>,
    #[serde(rename = "RequestURL")]
    pub request_url: String,
    #[serde(rename = "Parameters", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
    #[serde(rename = "Headers", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    /// Absent in ShareX 12 files, which sent multipart whenever there was a
    /// `FileFormName`.
    #[serde(rename = "Body", default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(rename = "Arguments", default, skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<String, String>,
    #[serde(rename = "FileFormName", default, skip_serializing_if = "Option::is_none")]
    pub file_form_name: Option<String>,
    /// ShareX 12: patterns that `$regex:index,group$` refers to, from 1.
    #[serde(rename = "RegexList", default, skip_serializing_if = "Vec::is_empty")]
    pub regex_list: Vec<String>,
    /// ShareX 12: `Text` (default) or `RedirectionURL`.
    #[serde(rename = "ResponseType", default, skip_serializing_if = "Option::is_none")]
    pub response_type: Option<String>,
    #[serde(rename = "URL", default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "ErrorMessage", default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// How the image travels.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BodyKind {
    Multipart,
    Binary,
}

/// A request ready to send: everything decided, nothing left to expand.
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// What came back, as the output syntax sees it.
pub struct Response<'a> {
    pub text: &'a str,
    /// The URL the response came from, after redirects.
    pub final_url: &'a str,
    pub headers: &'a [(String, String)],
}

const BOUNDARY: &str = "----snipdeckUploadBoundary5b1c2e7d";

impl Uploader {
    /// Read a `.sxcu` file and check it can do an image upload. The error is
    /// a sentence for the user.
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim_start_matches('\u{feff}');
        let mut up: Uploader = serde_json::from_str(text)
            .map_err(|_| anyhow!(crate::i18n::t("This file is not a ShareX custom uploader.")))?;
        up.name = up.name.trim().to_string();
        if up.name.is_empty() {
            up.name = up.domain();
        }
        up.validate()?;
        Ok(up)
    }

    /// The display name: what the file calls itself, else its host.
    pub fn display_name(&self) -> String {
        if self.name.is_empty() { self.domain() } else { self.name.clone() }
    }

    /// `host[:port]` of the request URL — what the consent prompt names.
    pub fn domain(&self) -> String {
        let rest = self
            .request_url
            .split_once("://")
            .map_or(self.request_url.as_str(), |(_, r)| r);
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let host = &rest[..end];
        // Credentials in the URL are not part of the name.
        host.rsplit_once('@').map_or(host, |(_, h)| h).to_string()
    }

    /// True for `https://`. The prompt says so when the image would travel
    /// in the clear — a self-hosted server on the LAN is a fair reason to.
    pub fn is_encrypted(&self) -> bool {
        self.request_url.trim().to_ascii_lowercase().starts_with("https://")
    }

    fn method(&self) -> String {
        self.request_method
            .as_deref()
            .unwrap_or("POST")
            .trim()
            .to_ascii_uppercase()
    }

    fn body_kind(&self) -> Result<BodyKind> {
        match self.body.as_deref().map(str::trim) {
            Some(b) if b.eq_ignore_ascii_case("MultipartFormData") => Ok(BodyKind::Multipart),
            Some(b) if b.eq_ignore_ascii_case("Binary") => Ok(BodyKind::Binary),
            // ShareX 12: multipart whenever a file field is named.
            None if self.file_form_name.as_deref().is_some_and(|f| !f.trim().is_empty()) => {
                Ok(BodyKind::Multipart)
            }
            other => Err(anyhow!(crate::i18n::t1(
                "It sends the image as \"{body}\"; Snipdeck can send multipart form data or a binary body.",
                "{body}",
                other.unwrap_or("None"),
            ))),
        }
    }

    fn validate(&self) -> Result<()> {
        let url = self.request_url.trim().to_ascii_lowercase();
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(anyhow!(crate::i18n::t(
                "It must upload to an http:// or https:// address."
            )));
        }
        if !matches!(self.method().as_str(), "POST" | "PUT" | "PATCH") {
            return Err(anyhow!(crate::i18n::t(
                "Only POST, PUT and PATCH uploads are supported."
            )));
        }
        if let Some(d) = &self.destination_type {
            let d = d.to_ascii_lowercase();
            if !(d.contains("image") || d.contains("file")) {
                return Err(anyhow!(crate::i18n::t(
                    "It uploads text or shortens links, not images."
                )));
            }
        }
        if self.body_kind()? == BodyKind::Multipart
            && self.file_form_name.as_deref().is_none_or(|f| f.trim().is_empty())
        {
            return Err(anyhow!(crate::i18n::t(
                "It does not say which form field carries the image (FileFormName)."
            )));
        }
        if let Some(r) = &self.response_type {
            if !(r.eq_ignore_ascii_case("Text") || r.eq_ignore_ascii_case("RedirectionURL")) {
                return Err(unsupported(&format!("ResponseType: {r}")));
            }
        }
        // Every template has to parse, and use only what is supported, now —
        // not on the first upload.
        let inputs = std::iter::once(self.request_url.as_str())
            .chain(self.parameters.values().map(String::as_str))
            .chain(self.headers.values().map(String::as_str))
            .chain(self.arguments.values().map(String::as_str));
        for t in inputs {
            for token in tokens(t)? {
                if let Token::Var(name, _) = token {
                    if name != "filename" {
                        return Err(unsupported(&format!("{{{name}}}")));
                    }
                }
            }
        }
        for t in [self.url.as_deref(), self.error_message.as_deref()].into_iter().flatten() {
            for token in tokens(t)? {
                if let Token::Var(name, arg) = token {
                    match name.as_str() {
                        "response" | "responseurl" | "header" | "json" => {}
                        "regex" => {
                            compile_regex(self, &arg)?;
                        }
                        _ => return Err(unsupported(&format!("{{{name}}}"))),
                    }
                }
            }
        }
        Ok(())
    }

    /// Build the request for one PNG.
    pub fn request(&self, png: &[u8], filename: &str) -> Result<Request> {
        let input = |t: &str| expand(t, &|name, _| (name == "filename").then(|| filename.to_string()));
        let mut url = input(self.request_url.trim())?;
        if !self.parameters.is_empty() {
            let query: Vec<String> = self
                .parameters
                .iter()
                .map(|(k, v)| Ok(format!("{}={}", encode(k), encode(&input(v)?))))
                .collect::<Result<_>>()?;
            url.push(if url.contains('?') { '&' } else { '?' });
            url.push_str(&query.join("&"));
        }
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .map(|(k, v)| Ok((k.clone(), input(v)?)))
            .collect::<Result<_>>()?;

        let body = match self.body_kind()? {
            BodyKind::Binary => {
                set_default(&mut headers, "Content-Type", "image/png");
                png.to_vec()
            }
            BodyKind::Multipart => {
                let field = self.file_form_name.as_deref().unwrap_or("file").trim();
                let mut body = Vec::with_capacity(png.len() + 512);
                for (k, v) in &self.arguments {
                    body.extend_from_slice(
                        format!(
                            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n",
                            quote(k),
                            input(v)?
                        )
                        .as_bytes(),
                    );
                }
                body.extend_from_slice(
                    format!(
                        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{}\"; \
                         filename=\"{}\"\r\nContent-Type: image/png\r\n\r\n",
                        quote(field),
                        quote(filename)
                    )
                    .as_bytes(),
                );
                body.extend_from_slice(png);
                body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
                // The boundary is ours, so this header is too, whatever the
                // file says.
                headers.retain(|(k, _)| !k.eq_ignore_ascii_case("Content-Type"));
                headers.push((
                    "Content-Type".into(),
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                ));
                body
            }
        };
        Ok(Request { method: self.method(), url, headers, body })
    }

    /// Read the link out of a successful response.
    pub fn link(&self, resp: &Response) -> Result<String> {
        let link = if self
            .response_type
            .as_deref()
            .is_some_and(|r| r.eq_ignore_ascii_case("RedirectionURL"))
        {
            resp.final_url.to_string()
        } else {
            match self.url.as_deref().map(str::trim) {
                Some(t) if !t.is_empty() => self.output(t, resp)?,
                _ => resp.text.trim().to_string(),
            }
        };
        let link = link.trim().to_string();
        if link.starts_with("https://") || link.starts_with("http://") {
            Ok(link)
        } else {
            Err(anyhow!(crate::i18n::t1(
                "The host answered without a link: {answer}",
                "{answer}",
                &shorten(if link.is_empty() { resp.text } else { &link }),
            )))
        }
    }

    /// The host's own explanation of a failed upload, if the file says where
    /// to find it.
    pub fn error_text(&self, resp: &Response) -> Option<String> {
        let t = self.error_message.as_deref()?.trim();
        if t.is_empty() {
            return None;
        }
        self.output(t, resp).ok().filter(|s| !s.trim().is_empty())
    }

    fn output(&self, template: &str, resp: &Response) -> Result<String> {
        let json = std::cell::OnceCell::new();
        expand(template, &|name, arg| match name {
            "response" => Some(resp.text.to_string()),
            "responseurl" => Some(resp.final_url.to_string()),
            "header" => resp
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(arg))
                .map(|(_, v)| v.clone()),
            "json" => {
                let v = json.get_or_init(|| serde_json::from_str::<serde_json::Value>(resp.text).ok());
                v.as_ref().and_then(|v| json_path(v, arg))
            }
            "regex" => {
                let (re, group) = compile_regex(self, arg).ok()?;
                let caps = re.captures(resp.text)?;
                let m = match group.parse::<usize>() {
                    Ok(i) => caps.get(i),
                    Err(_) if group.is_empty() => caps.get(1).or_else(|| caps.get(0)),
                    Err(_) => caps.name(&group),
                };
                m.map(|m| m.as_str().to_string())
            }
            _ => None,
        })
    }
}

fn unsupported(what: &str) -> anyhow::Error {
    anyhow!(crate::i18n::t1(
        "It uses {what}, which Snipdeck does not support.",
        "{what}",
        what
    ))
}

/// `{regex:pattern|group}`, or ShareX 12's `index,group` into `RegexList`.
fn compile_regex(up: &Uploader, arg: &str) -> Result<(regex::Regex, String)> {
    let (pattern, group) = match arg.split_once(',') {
        Some((i, g)) if i.trim().parse::<usize>().is_ok() && !up.regex_list.is_empty() => {
            let i: usize = i.trim().parse().unwrap_or(0);
            let p = up
                .regex_list
                .get(i.wrapping_sub(1))
                .ok_or_else(|| unsupported(&format!("$regex:{arg}$")))?;
            (p.clone(), g.trim().to_string())
        }
        _ => match arg.rsplit_once('|') {
            Some((p, g)) => (p.to_string(), g.trim().to_string()),
            None => (arg.to_string(), String::new()),
        },
    };
    let re = regex::Regex::new(&pattern).map_err(|_| unsupported(&format!("{{regex:{arg}}}")))?;
    Ok((re, group))
}

#[derive(Debug, PartialEq)]
enum Token {
    Text(String),
    Var(String, String),
}

/// Names that ShareX's syntax uses, so a `$` or `{` that is not one of them
/// stays literal text (a URL may well contain either).
const KNOWN: &[&str] = &[
    "response", "responseurl", "header", "json", "xml", "regex", "filename", "random",
    "select", "prompt", "inputbox", "outputbox", "base64", "uniqueid", "input",
];

/// Split a template into text and `{name:arg}` / `$name:arg$` variables.
/// `\` escapes the next character, as in ShareX.
fn tokens(t: &str) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    let mut text = String::new();
    let chars: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            text.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '{' || c == '$' {
            let close = if c == '{' { '}' } else { '$' };
            if let Some(len) = variable_len(&chars[i + 1..], close) {
                let inner: String = chars[i + 1..i + 1 + len].iter().collect();
                let (name, arg) = inner.split_once(':').unwrap_or((&inner, ""));
                let name = name.trim().to_ascii_lowercase();
                if KNOWN.contains(&name.as_str()) {
                    if !text.is_empty() {
                        out.push(Token::Text(std::mem::take(&mut text)));
                    }
                    out.push(Token::Var(name, arg.to_string()));
                    i += len + 2;
                    continue;
                }
            }
        }
        text.push(c);
        i += 1;
    }
    if !text.is_empty() {
        out.push(Token::Text(text));
    }
    Ok(out)
}

/// Length of a variable body up to its closing character, honouring nested
/// braces (a JSON path or a regex may contain them) and escapes.
fn variable_len(rest: &[char], close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            '\\' => i += 1,
            '{' if close == '}' => depth += 1,
            c if c == close => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Fill a template's variables. A variable with no value is an error: an
/// upload that half-filled its URL would send the image somewhere else.
fn expand(t: &str, value: &dyn Fn(&str, &str) -> Option<String>) -> Result<String> {
    let mut out = String::new();
    for token in tokens(t)? {
        match token {
            Token::Text(s) => out.push_str(&s),
            Token::Var(name, arg) => match value(&name, &arg) {
                Some(v) => out.push_str(&v),
                None => {
                    let shown = if arg.is_empty() { name } else { format!("{name}:{arg}") };
                    return Err(anyhow!(crate::i18n::t1(
                        "The host's answer has no {what}.",
                        "{what}",
                        &format!("{{{shown}}}")
                    )));
                }
            },
        }
    }
    Ok(out)
}

/// `data.link`, `files[0].url`, `$.image.url`, `[0]` — the paths `.sxcu`
/// files use.
fn json_path(root: &serde_json::Value, path: &str) -> Option<String> {
    let path = path.trim();
    let path = path.strip_prefix('$').unwrap_or(path);
    let mut v = root;
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '.' => i += 1,
            '[' => {
                let end = i + chars[i..].iter().position(|&c| c == ']')?;
                let inner: String = chars[i + 1..end].iter().collect();
                let inner = inner.trim();
                v = if let Ok(n) = inner.parse::<usize>() {
                    v.get(n)?
                } else {
                    v.get(inner.trim_matches(|c| c == '\'' || c == '"'))?
                };
                i = end + 1;
            }
            _ => {
                let end = chars[i..]
                    .iter()
                    .position(|&c| c == '.' || c == '[')
                    .map_or(chars.len(), |p| i + p);
                let key: String = chars[i..end].iter().collect();
                v = v.get(key.as_str())?;
                i = end;
            }
        }
    }
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn set_default(headers: &mut Vec<(String, String)>, key: &str, value: &str) {
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(key)) {
        headers.push((key.to_string(), value.to_string()));
    }
}

/// Percent-encode a query component (RFC 3986 unreserved characters kept).
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A multipart name or filename: no quote or line break can end it early.
fn quote(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\r' | '\n'))
        .map(|c| if c == '"' { '\'' } else { c })
        .collect()
}

fn shorten(s: &str) -> String {
    let s = s.trim();
    let cut: String = s.chars().take(160).collect();
    if cut.len() < s.len() { format!("{cut}…") } else { cut }
}

/// Why a request never got an answer, in the operating system's words —
/// "Connection refused", not ureq's "http://…: Connection Failed: Connect
/// error: Connection refused (os error 111)".
pub fn net_reason(e: &ureq::Error) -> String {
    use std::error::Error as _;
    match e {
        ureq::Error::Transport(t) => {
            let mut root: Option<&(dyn std::error::Error + 'static)> = t.source();
            while let Some(next) = root.and_then(|r| r.source()) {
                root = Some(next);
            }
            root.map(|r| r.to_string())
                .or_else(|| t.message().map(str::to_string))
                .unwrap_or_else(|| t.kind().to_string())
        }
        ureq::Error::Status(code, _) => format!("HTTP {code}"),
    }
}

/// Send one upload and return the link. Blocking; run it off the UI thread.
pub fn send(agent: &ureq::Agent, up: &Uploader, png: &[u8], filename: &str) -> Result<String> {
    let req = up.request(png, filename)?;
    let mut call = agent.request(&req.method, &req.url);
    for (k, v) in &req.headers {
        call = call.set(k, v);
    }
    let (ok, resp) = match call.send_bytes(&req.body) {
        Ok(r) => (true, r),
        Err(ureq::Error::Status(_, r)) => (false, r),
        Err(e) => {
            return Err(anyhow!(crate::i18n::t1(
                "Could not reach {host}: {e}",
                "{host}",
                &up.domain()
            )
            .replace("{e}", &net_reason(&e))))
        }
    };
    let status = resp.status();
    let final_url = resp.get_url().to_string();
    let headers: Vec<(String, String)> = resp
        .headers_names()
        .into_iter()
        .filter_map(|n| resp.header(&n).map(|v| (n.clone(), v.to_string())))
        .collect();
    let text = resp.into_string().unwrap_or_default();
    let view = Response { text: &text, final_url: &final_url, headers: &headers };
    if ok {
        up.link(&view)
    } else {
        let why = up.error_text(&view).unwrap_or_else(|| shorten(&text));
        Err(anyhow!("HTTP {status}: {why}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZIPLINE: &str = r#"{
      "Version": "14.1.0", "Name": "Zipline", "DestinationType": "ImageUploader, FileUploader",
      "RequestMethod": "POST", "RequestURL": "https://zip.example.com/api/upload",
      "Headers": { "Authorization": "secret-token", "Format": "RANDOM" },
      "Body": "MultipartFormData", "FileFormName": "file",
      "URL": "{json:files[0]}", "ErrorMessage": "{json:error}"
    }"#;

    const XBACKBONE: &str = r#"{
      "Version": "13.2.1", "Name": "XBackBone", "DestinationType": "ImageUploader, TextUploader, FileUploader",
      "RequestMethod": "POST", "RequestURL": "https://xbb.example.org/upload",
      "Body": "MultipartFormData", "Arguments": { "token": "token_abc" },
      "FileFormName": "upload", "URL": "{json:url}", "DeletionURL": "{json:url}/delete/{json:delete_url}"
    }"#;

    /// ShareX 12: `RequestType`, no `Body`, `$…$` syntax and a `RegexList`.
    const CHEVERETO_OLD: &str = "\u{feff}{
      \"Name\": \"Chevereto\", \"DestinationType\": \"ImageUploader\", \"RequestType\": \"POST\",
      \"RequestURL\": \"https://img.example.net/api/1/upload\",
      \"FileFormName\": \"source\", \"Arguments\": { \"key\": \"k1\", \"format\": \"json\" },
      \"ResponseType\": \"Text\", \"RegexList\": [\"\\\"url\\\":\\\"([^\\\"]+)\\\"\"],
      \"URL\": \"$json:image.url$\", \"ThumbnailURL\": \"$regex:1,1$\"
    }";

    fn resp<'a>(text: &'a str, headers: &'a [(String, String)]) -> Response<'a> {
        Response { text, final_url: "https://final.example/x", headers }
    }

    #[test]
    fn real_world_files_parse() {
        let z = Uploader::parse(ZIPLINE).unwrap();
        assert_eq!(z.display_name(), "Zipline");
        assert_eq!(z.domain(), "zip.example.com");
        assert!(z.is_encrypted());
        Uploader::parse(XBACKBONE).unwrap();
        let c = Uploader::parse(CHEVERETO_OLD).unwrap();
        assert_eq!(c.method(), "POST");
        assert_eq!(c.body_kind().unwrap(), BodyKind::Multipart);
    }

    #[test]
    fn a_multipart_request_carries_the_arguments_and_the_image() {
        let up = Uploader::parse(XBACKBONE).unwrap();
        let req = up.request(b"PNGDATA", "snip.png").unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "https://xbb.example.org/upload");
        let body = String::from_utf8_lossy(&req.body);
        assert!(body.contains("name=\"token\"\r\n\r\ntoken_abc\r\n"), "{body}");
        assert!(body.contains("name=\"upload\"; filename=\"snip.png\"\r\nContent-Type: image/png\r\n\r\nPNGDATA\r\n"));
        assert!(body.ends_with(&format!("--{BOUNDARY}--\r\n")));
        let ct = req.headers.iter().find(|(k, _)| k == "Content-Type").unwrap();
        assert_eq!(ct.1, format!("multipart/form-data; boundary={BOUNDARY}"));
    }

    #[test]
    fn parameters_become_an_encoded_query_and_filename_is_filled_in() {
        let up = Uploader::parse(
            r#"{"RequestURL":"https://h.example/up?x=1","Body":"Binary",
                "Parameters":{"name":"{filename}","q":"a b&c"},"Headers":{"X-Key":"k"}}"#,
        )
        .unwrap();
        let req = up.request(b"P", "my snip.png").unwrap();
        assert_eq!(req.url, "https://h.example/up?x=1&name=my%20snip.png&q=a%20b%26c");
        assert_eq!(req.body, b"P");
        assert!(req.headers.contains(&("Content-Type".into(), "image/png".into())));
        assert!(req.headers.contains(&("X-Key".into(), "k".into())));
    }

    #[test]
    fn the_link_is_read_with_either_syntax() {
        let z = Uploader::parse(ZIPLINE).unwrap();
        let r = resp(r#"{"files":["https://zip.example.com/u/a.png"]}"#, &[]);
        assert_eq!(z.link(&r).unwrap(), "https://zip.example.com/u/a.png");

        let c = Uploader::parse(CHEVERETO_OLD).unwrap();
        let r = resp(r#"{"image":{"url":"https://img.example.net/i/b.png"}}"#, &[]);
        assert_eq!(c.link(&r).unwrap(), "https://img.example.net/i/b.png");
    }

    #[test]
    fn an_empty_url_means_the_body_is_the_link() {
        let up = Uploader::parse(r#"{"RequestURL":"https://h.example/","FileFormName":"f"}"#).unwrap();
        assert_eq!(up.link(&resp("  https://h.example/x.png\n", &[])).unwrap(), "https://h.example/x.png");
        assert!(up.link(&resp("Error: too large", &[])).is_err());
    }

    #[test]
    fn headers_regex_and_redirects_resolve() {
        let h = vec![("Location".to_string(), "https://h.example/l.png".to_string())];
        let up = Uploader::parse(r#"{"RequestURL":"https://h.example/","FileFormName":"f","URL":"{header:location}"}"#).unwrap();
        assert_eq!(up.link(&resp("", &h)).unwrap(), "https://h.example/l.png");

        let up = Uploader::parse(r#"{"RequestURL":"https://h.example/","FileFormName":"f","URL":"https://h.example/{regex:id=(\\w+)|1}.png"}"#).unwrap();
        assert_eq!(up.link(&resp("ok id=abc12", &[])).unwrap(), "https://h.example/abc12.png");

        let up = Uploader::parse(r#"{"RequestURL":"https://h.example/","FileFormName":"f","ResponseType":"RedirectionURL"}"#).unwrap();
        assert_eq!(up.link(&resp("", &[])).unwrap(), "https://final.example/x");
    }

    #[test]
    fn the_hosts_error_message_is_used() {
        let z = Uploader::parse(ZIPLINE).unwrap();
        let r = resp(r#"{"error":"file too large"}"#, &[]);
        assert_eq!(z.error_text(&r).as_deref(), Some("file too large"));
    }

    #[test]
    fn what_cannot_work_is_refused_at_import() {
        for bad in [
            r#"{"RequestURL":"ftp://h/","FileFormName":"f"}"#,
            r#"{"RequestURL":"https://h/","RequestMethod":"GET","FileFormName":"f"}"#,
            r#"{"RequestURL":"https://h/","Body":"JSON","Data":"{}"}"#,
            r#"{"RequestURL":"https://h/","Body":"MultipartFormData"}"#,
            r#"{"RequestURL":"https://h/","DestinationType":"URLShortener","FileFormName":"f"}"#,
            r#"{"RequestURL":"https://h/","FileFormName":"f","Arguments":{"k":"{prompt:Key}"}}"#,
            r#"{"RequestURL":"https://h/","FileFormName":"f","URL":"{xml:/a}"}"#,
            r#"{"RequestURL":"https://h/","FileFormName":"f","ResponseType":"LocationHeader"}"#,
            "not json",
        ] {
            assert!(Uploader::parse(bad).is_err(), "accepted: {bad}");
        }
    }

    #[test]
    fn dollars_and_braces_that_are_not_syntax_stay_text() {
        let up = Uploader::parse(r#"{"RequestURL":"https://h.example/a$b/{x}","Body":"Binary"}"#).unwrap();
        assert_eq!(up.request(b"", "s.png").unwrap().url, "https://h.example/a$b/{x}");
    }

    #[test]
    fn domain_drops_credentials_and_path() {
        let up = Uploader::parse(r#"{"RequestURL":"http://u:p@10.0.0.2:8080/up","Body":"Binary"}"#).unwrap();
        assert_eq!(up.domain(), "10.0.0.2:8080");
        assert!(!up.is_encrypted());
        assert_eq!(up.display_name(), "10.0.0.2:8080");
    }

    /// The whole path once, over a real socket: request on the wire, the
    /// answer read back, the link out of it.
    #[test]
    fn uploads_to_a_loopback_server() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            // Read the head, then exactly Content-Length bytes of body.
            let (head_end, len) = loop {
                let n = s.read(&mut chunk).unwrap();
                buf.extend_from_slice(&chunk[..n]);
                if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&buf[..p]).to_ascii_lowercase();
                    let len: usize = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .map(|v| v.trim().parse().unwrap())
                        .unwrap_or(0);
                    break (p + 4, len);
                }
            };
            while buf.len() < head_end + len {
                let n = s.read(&mut chunk).unwrap();
                buf.extend_from_slice(&chunk[..n]);
            }
            let reply = br#"{"files":["https://zip.example.com/u/ok.png"]}"#;
            write!(
                s,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                reply.len()
            )
            .unwrap();
            s.write_all(reply).unwrap();
            String::from_utf8_lossy(&buf).to_string()
        });

        let up = Uploader {
            request_url: format!("http://127.0.0.1:{port}/api/upload"),
            ..Uploader::parse(ZIPLINE).unwrap()
        };
        let agent = ureq::AgentBuilder::new().build();
        let link = send(&agent, &up, b"\x89PNG-bytes", "snip.png").unwrap();
        assert_eq!(link, "https://zip.example.com/u/ok.png");

        let wire = server.join().unwrap();
        assert!(wire.starts_with("POST /api/upload HTTP/1.1\r\n"), "{wire}");
        assert!(wire.to_ascii_lowercase().contains("\r\nauthorization: secret-token\r\n"));
        assert!(wire.contains("name=\"file\"; filename=\"snip.png\""));
        assert!(wire.contains("\u{fffd}PNG-bytes"));
    }
}
