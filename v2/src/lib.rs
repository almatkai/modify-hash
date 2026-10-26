pub mod core;
pub mod media;
pub mod safe_fs;
use core::{Draft, Outcome};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tezbar_extension_sdk::{
    unchanged, view, BoxFuture, CommandConcurrency, CoreResult, Extension, ExtensionContext,
};
use uuid::Uuid;
pub const ID: &str = "com.tezbar.native.modify-hash";
pub const COMMANDS: &[&str] = &["modifyHash", "restoreHash", "zipCompress", "zipExtract"];
type Sessions = Arc<Mutex<HashMap<String, Session>>>;
#[derive(Clone)]
struct SelectionCandidate {
    request: String,
    revision: u64,
    paths: Value,
    accept: String,
}
#[derive(Clone)]
struct Session {
    token: String,
    revision: u64,
    fields: Value,
    execute: Option<String>,
    selection: Option<String>,
    candidate: Option<SelectionCandidate>,
    reset: Option<String>,
    status: String,
    outcomes: Vec<Outcome>,
    finished: bool,
}
fn fresh() -> String {
    Uuid::new_v4().to_string()
}
fn fields(d: &Draft) -> Value {
    let mut v = serde_json::to_value(d).unwrap();
    v["paths"] = json!(serde_json::to_string(&d.paths).unwrap());
    v
}
fn draft(v: &Value) -> Result<Draft, String> {
    let mut v = v.clone();
    let paths = v["paths"]
        .as_str()
        .ok_or("Paths must be a JSON array of absolute strings")?;
    let paths: Value = serde_json::from_str(paths)
        .map_err(|_| "Paths must be a JSON array of absolute strings")?;
    v["paths"] = paths;
    serde_json::from_value(v).map_err(|e| format!("Invalid complete form: {e}"))
}
impl Session {
    fn initial(token: String, fields: Value) -> Self {
        Self { token: token.clone(), revision: 0, fields, execute: None,
            selection: Some(format!("{token}:select:0")), candidate: None, reset: None,
            status: "Choose files or directories, then submit explicitly. Paths are a JSON array; use Finder/Explorer Selection to fill it.".into(), outcomes: vec![], finished: false }
    }
    fn renew(&mut self, command: &str) {
        self.execute = None;
        if self.finished {
            return;
        }
        match draft(&self.fields).and_then(|d| d.validate(command)) {
            Ok(()) => {
                self.execute = Some(format!("{}:run:{}:{}", self.token, self.revision, fresh()));
                self.status = "Ready. Submit performs filesystem changes.".into();
            }
            Err(e) => self.status = e,
        }
    }
}
#[derive(Default)]
pub struct Adapter {
    sessions: Sessions,
    files: tokio::sync::Mutex<()>,
}
struct ActionGuard {
    sessions: Sessions,
    id: String,
    done: bool,
    selection_command: Option<String>,
}
impl Drop for ActionGuard {
    fn drop(&mut self) {
        if !self.done {
            if let Some(s) = self.sessions.lock().unwrap().get_mut(&self.id) {
                if let Some(command) = &self.selection_command {
                    s.candidate = None;
                    s.selection = Some(format!("{}:select:{}", s.token, fresh()));
                    s.renew(command);
                    s.status = "Selection interrupted. Draft preserved; request selection again or submit the current draft explicitly.".into();
                    return;
                }
                s.status = "Interrupted or response lost. Recorded committed items remain committed; remaining items were not automatically retried. Check outputs before starting a new batch. No crash durability is promised.".into();
                s.finished = true;
                s.execute = None;
                s.selection = None;
                s.reset = Some(format!("{}:new:{}", s.token, fresh()));
            }
        }
    }
}
fn bounded_view(v: Value) -> CoreResult {
    if serde_json::to_vec(&v).map_err(|e| e.to_string())?.len() > 12 * 1024 * 1024 {
        Err("View exceeds 12 MiB budget; use a smaller batch. Recorded filesystem results remain committed.".into())
    } else {
        Ok(v)
    }
}
fn render(command: &str, s: &Session) -> CoreResult {
    let title = match command {
        "modifyHash" => "Modify Hash",
        "restoreHash" => "Restore Hash",
        "zipCompress" => "Zip Compress",
        _ => "Zip Extract",
    };
    let mut actions = vec![];
    if let Some(id) = &s.execute {
        actions.push(json!({"id":id,"title":title,"kind":"submit-form"}));
    }
    if let Some(id) = &s.selection {
        if !s.finished {
            actions.push(json!({"id":id,"title":"Use Finder/Explorer Selection","kind":"custom"}));
        }
    }
    if let Some(id) = &s.reset {
        actions.push(json!({"id":id,"title":"Start a New Batch","kind":"custom"}));
    }
    if s.finished {
        let text = serde_json::to_string_pretty(&json!({"status":s.status,"results":s.outcomes}))
            .map_err(|e| e.to_string())?;
        let mut markdown = String::new();
        for line in text.lines() {
            markdown.push_str("    ");
            markdown.push_str(line);
            markdown.push('\n');
        }
        return bounded_view(view(
            title,
            json!({"type":"Detail","props":{"markdown":markdown}}),
            actions,
        ));
    }
    let mut children =
        vec![json!({"type":"Form.Description","props":{"title":"Batch status","text":s.status}})];
    for (name, label, kind) in [
        ("paths", "Absolute paths (JSON array)", "Form.TextArea"),
        ("showMD5Log", "Show MD5 Log", "Form.Checkbox"),
        ("enableVideo", "Video", "Form.Checkbox"),
        ("enableAudio", "Audio", "Form.Checkbox"),
        ("enableImage", "Image", "Form.Checkbox"),
        (
            "enableZipPassword",
            "Enable Zip Password (unsupported)",
            "Form.Checkbox",
        ),
        ("zipPassword", "Zip Password", "Form.PasswordField"),
    ] {
        children.push(json!({"type":kind,"props":{"id":name,"title":label,"label":label,"value":s.fields[name],"actionId":format!("{}:field",s.token)}}));
    }
    bounded_view(view(
        title,
        json!({"type":"Form","props":{"draftRevision":s.revision,"draftOwner":s.token,"requiresDraftRevision":true,"selectionActionId":s.selection,"selectionCandidate":s.candidate.as_ref().map(|c| json!({"requestId":c.request,"revision":c.revision,"paths":c.paths,"acceptActionId":c.accept}))},"children":children}),
        actions,
    ))
}
fn form_snapshot(p: &Value) -> Result<(u64, Value), String> {
    let mut v = p["formValues"].as_object().ok_or("Complete formValues and _draftRevision are required (apply the documented renderer prerequisite)")?.clone();
    let r = v
        .remove("_draftRevision")
        .and_then(|v| v.as_u64())
        .filter(|n| *n <= 9_007_199_254_740_991)
        .ok_or("Missing/invalid monotonic _draftRevision")?;
    if serde_json::to_vec(&v).map_err(|e| e.to_string())?.len() > 128 * 1024 {
        return Err("Form exceeds 128 KiB".into());
    }
    Ok((r, Value::Object(v)))
}
impl Extension for Adapter {
    fn id(&self) -> &'static str {
        ID
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn commands(&self) -> &'static [&'static str] {
        COMMANDS
    }
    fn concurrency(&self, _: &str) -> CommandConcurrency {
        CommandConcurrency::Serialize
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        op: &'a str,
        p: &'a Value,
        state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            let sid = p["sessionId"]
                .as_str()
                .ok_or("Session identity missing")?
                .to_owned();
            if op == "extension:dispose-session" {
                self.sessions.lock().unwrap().remove(&sid);
                return Ok(unchanged());
            }
            if op == "extension:run-command" {
                let token = fresh();
                let f = fields(&Draft::defaults(&p["preferences"]));
                *state = json!({"token":token,"fields":f});
                // No registry allocation or awaits on open: a dropped open needs no dispose.
                return render(command, &Session::initial(token, f));
            }
            let token = state["token"]
                .as_str()
                .ok_or("Session was not opened")?
                .to_owned();
            if op != "extension:invoke-action" {
                return match self.sessions.lock().unwrap().get(&sid) {
                    Some(s) => render(command, s),
                    None => render(command, &Session::initial(token, state["fields"].clone())),
                };
            }
            let id = p["actionId"].as_str().ok_or("Missing actionId")?;
            let (mut guard, frozen) = {
                let mut sessions = self.sessions.lock().unwrap();
                if !sessions.contains_key(&sid) && sessions.len() >= 128 {
                    return Err("Adapter capacity reached".into());
                }
                let s = sessions
                    .entry(sid.clone())
                    .or_insert_with(|| Session::initial(token, state["fields"].clone()));
                if id == format!("{}:field", s.token) {
                    // Invalidate outside staged SDK state, including invalid intermediate drafts.
                    match form_snapshot(p) {
                        Ok((revision, fields)) if revision > s.revision => {
                            s.candidate = None;
                            s.revision = revision;
                            s.fields = fields;
                            s.renew(command);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            s.execute = None;
                            s.candidate = None;
                            s.status = e;
                        }
                    }
                    return render(command, s);
                }
                if s.candidate.as_ref().is_some_and(|c| c.accept == id) && !s.finished {
                    // One-use, effect-free candidate admission. Only the renderer owns
                    // revision increments; selection never borrows that namespace.
                    let c = s.candidate.take().unwrap();
                    s.execute = None;
                    let mut expected = s.fields.clone();
                    expected["paths"] = c.paths;
                    match form_snapshot(p) {
                        Ok((r, f)) if s.revision == c.revision
                            && r == c.revision + 1 && f == expected => {
                                s.revision = r;
                                s.fields = f;
                                s.renew(command);
                            }
                        _ => s.status = "Selection candidate expired; edit the draft or request selection again.".into(),
                    }
                    return render(command, s);
                }
                if s.reset.as_deref() == Some(id) {
                    let f = fields(&Draft::defaults(&p["preferences"]));
                    let token = fresh();
                    *state = json!({"token":token,"fields":f});
                    *s = Session::initial(token, f);
                    return render(command, s);
                }
                if s.selection.as_deref() == Some(id) && !s.finished {
                    s.selection = None;
                    s.execute = None;
                    s.candidate = None;
                    match form_snapshot(p) {
                        Ok((r, f)) if r > s.revision || (r == s.revision && f == s.fields) => {
                            s.revision = r;
                            s.fields = f;
                        }
                        _ => {
                            s.selection = Some(format!("{}:select:{}", s.token, fresh()));
                            s.status =
                                "Selection requires the current complete versioned draft.".into();
                            return render(command, s);
                        }
                    }
                    s.status = "Reading explicitly requested selection…".into();
                    (
                        ActionGuard {
                            sessions: self.sessions.clone(),
                            id: sid.clone(),
                            done: false,
                            selection_command: Some(command.to_owned()),
                        },
                        None,
                    )
                } else if s.execute.as_deref() == Some(id) && !s.finished {
                    // Consume before validation or await; SDK rollback cannot restore a token.
                    s.execute = None;
                    let (r, f) = match form_snapshot(p) {
                        Ok(v) => v,
                        Err(e) => {
                            s.status = e;
                            return render(command, s);
                        }
                    };
                    if r != s.revision || f != s.fields {
                        s.status="Draft changed; wait for the latest field response and edit again before submitting".into();
                        return render(command, s);
                    }
                    let d = match draft(&f).and_then(|d| {
                        d.validate(command)?;
                        Ok(d)
                    }) {
                        Ok(d) => d,
                        Err(e) => {
                            s.status = e;
                            return render(command, s);
                        }
                    };
                    s.selection = None;
                    s.finished = true;
                    s.status =
                        "Batch in progress. Completed items are recorded immediately.".into();
                    (
                        ActionGuard {
                            sessions: self.sessions.clone(),
                            id: sid.clone(),
                            done: false,
                            selection_command: None,
                        },
                        Some(d),
                    )
                } else {
                    return render(command, s);
                }
            };
            if let Some(d) = frozen {
                // Never queue overlapping filesystem effects across sessions.
                let _files = match self.files.try_lock() {
                    Ok(l) => l,
                    Err(_) => {
                        let mut map = self.sessions.lock().unwrap();
                        let s = map.get_mut(&sid).ok_or("Disposed")?;
                        s.status="Another filesystem batch is active; nothing from this batch was attempted. Start a new batch explicitly.".into();
                        s.reset = Some(format!("{}:new:{}", s.token, fresh()));
                        guard.done = true;
                        return render(command, s);
                    }
                };
                let result = core::run(ctx, command, &d, |mut outcome| {
                    if outcome.message.len() > 2048 {
                        outcome.message = "Operation failed: diagnostic exceeded 2048 bytes".into();
                    }
                    if let Some(s) = self.sessions.lock().unwrap().get_mut(&sid) {
                        s.outcomes.push(outcome);
                    }
                })
                .await;
                let mut map = self.sessions.lock().unwrap();
                let s = map.get_mut(&sid).ok_or("Disposed")?;
                s.status=match result { Ok(())=>"Batch finished. Review per-item states; failures are never reported as successes. Starting a new batch is a new deliberate operation.".into(),Err(e)=>format!("Batch interrupted: {e}. Committed items remain committed; no automatic replay.") };
                s.reset = Some(format!("{}:new:{}", s.token, fresh()));
                guard.done = true;
                render(command, s)
            } else {
                let result = ctx
                    .host("host.selection.read", json!({"kind":"files"}))
                    .await;
                let mut map = self.sessions.lock().unwrap();
                let s = map.get_mut(&sid).ok_or("Disposed")?;
                let paths = result.and_then(|v| {
                    let a = v
                        .as_array()
                        .ok_or("Selection must be an array of absolute strings")?;
                    if a.len() > 128 {
                        return Err("Selection exceeds 128 paths".into());
                    }
                    let mut out = vec![];
                    for p in a {
                        let p = p
                            .as_str()
                            .ok_or("Selection item must be an absolute string")?;
                        safe_fs::absolute(std::path::Path::new(p))?;
                        out.push(p);
                    }
                    Ok(json!(
                        serde_json::to_string(&out).map_err(|e| e.to_string())?
                    ))
                });
                s.selection = Some(format!("{}:select:{}", s.token, fresh()));
                match paths {
                    Ok(paths) => {
                        s.candidate = Some(SelectionCandidate {
                            request: id.to_owned(),
                            revision: s.revision,
                            paths,
                            accept: format!("{}:accept-selection:{}", s.token, fresh()),
                        });
                        s.status = "Selection received; admitting only if the requesting draft is still current.".into();
                    }
                    Err(e) => {
                        s.renew(command);
                        s.status = format!("Selection failed: {e}");
                    }
                }
                guard.done = true;
                render(command, s)
            }
        })
    }
}
