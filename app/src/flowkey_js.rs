use rquickjs::loader::{Loader, Resolver};
use rquickjs::{CatchResultExt, Context, Ctx, Function, Module, Object, Runtime};
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

const HOST: &str = include_str!("../assets/flowkey-host.js");
const MEMORY_LIMIT: usize = 512 * 1024 * 1024;
const TURN_LIMIT: Duration = Duration::from_secs(10);

pub enum Inbox {
    Message(String),
    Stop,
}

pub struct Worker {
    sender: Sender<Inbox>,
    stopped: Arc<AtomicBool>,
}

impl Worker {
    pub fn send(&self, message: &serde_json::Value) {
        let _ = self.sender.send(Inbox::Message(message.to_string()));
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = self.sender.send(Inbox::Stop);
    }
}

struct Entry(String);

impl Resolver for Entry {
    fn resolve<'js>(&mut self, _: &Ctx<'js>, _base: &str, name: &str, _: Option<rquickjs::loader::ImportAttributes<'js>>) -> rquickjs::Result<String> {
        if name == self.0 {
            Ok(name.to_owned())
        } else {
            Err(rquickjs::Error::new_resolving("flowkey-host", name))
        }
    }
}

impl Loader for Entry {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str, _: Option<rquickjs::loader::ImportAttributes<'js>>) -> rquickjs::Result<Module<'js>> {
        if name != self.0 {
            return Err(rquickjs::Error::new_loading(name));
        }
        Module::declare(ctx.clone(), name, std::fs::read(name)?)
    }
}

fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes.min(65_536)];
    let _ = getrandom::fill(&mut buffer);
    buffer.iter().map(|b| format!("{b:02x}")).collect()
}

fn report(ctx: &Ctx, what: &str, result: rquickjs::Result<()>, log: &dyn Fn(&str, String)) {
    if let Err(error) = result.catch(ctx) {
        log("error", format!("{what}: {error}"));
    }
}

pub fn spawn(name: String, entry: String, post: impl Fn(String) + Send + 'static, log: impl Fn(&str, String) + Send + 'static) -> Worker {
    let (sender, receiver) = channel::<Inbox>();
    let stopped = Arc::new(AtomicBool::new(false));
    let (flag, halted) = (stopped.clone(), stopped.clone());
    let _ = std::thread::Builder::new().name(format!("flowkey:{name}")).spawn(move || {
        let Ok(runtime) = Runtime::new() else { return log("error", "could not start QuickJS".into()) };
        runtime.set_memory_limit(MEMORY_LIMIT);
        runtime.set_max_stack_size(1024 * 1024);
        runtime.set_loader(Entry(entry.clone()), Entry(entry));
        let turn = Arc::new(AtomicU64::new(0));
        let (watch, origin) = (turn.clone(), Instant::now());
        runtime.set_interrupt_handler(Some(Box::new(move || {
            let started = watch.load(Ordering::Relaxed);
            flag.load(Ordering::Relaxed) || (started > 0 && origin.elapsed().as_millis() as u64 > started + TURN_LIMIT.as_millis() as u64)
        })));
        let Ok(context) = Context::full(&runtime) else { return log("error", "could not create a QuickJS context".into()) };
        let log: Rc<dyn Fn(&str, String)> = Rc::new(log);
        let timers: Rc<RefCell<BinaryHeap<Reverse<(Instant, u64, u64)>>>> = Rc::default();
        let sequence = Rc::new(RefCell::new(0u64));
        let started = context.with(|ctx| -> rquickjs::Result<()> {
            let rust = Object::new(ctx.clone())?;
            rust.set("post", Function::new(ctx.clone(), move |text: String| post(text))?)?;
            let (queue, seq) = (timers.clone(), sequence.clone());
            rust.set(
                "timer",
                Function::new(ctx.clone(), move |id: f64, ms: f64| {
                    let mut n = seq.borrow_mut();
                    *n += 1;
                    queue.borrow_mut().push(Reverse((Instant::now() + Duration::from_millis(ms.clamp(0.0, 2_147_483_647.0) as u64), *n, id as u64)));
                })?,
            )?;
            let logger = log.clone();
            rust.set("log", Function::new(ctx.clone(), move |level: String, message: String| logger(&level, message))?)?;
            rust.set("random", Function::new(ctx.clone(), |bytes: usize| random_hex(bytes))?)?;
            ctx.globals().set("__rust", rust)?;
            Module::evaluate(ctx.clone(), "flowkey-host", HOST)?.finish::<()>()
        });
        if let Err(error) = context.with(|ctx| started.catch(&ctx).map_err(|e| e.to_string())) {
            return log("error", format!("the FlowKey host failed to start: {error}"));
        }
        let mark = |on: bool| turn.store(if on { origin.elapsed().as_millis() as u64 + 1 } else { 0 }, Ordering::Relaxed);
        loop {
            mark(true);
            loop {
                match runtime.execute_pending_job() {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(error) => {
                        let message = error.0.with(|ctx| format!("{:?}", ctx.catch()));
                        log("error", format!("unhandled job error: {message}"));
                    }
                }
            }
            if halted.load(Ordering::Relaxed) {
                break;
            }
            let now = Instant::now();
            let due: Vec<u64> = std::iter::from_fn(|| {
                let mut queue = timers.borrow_mut();
                (queue.peek().is_some_and(|Reverse((at, _, _))| *at <= now)).then(|| queue.pop().map(|Reverse((_, _, id))| id)).flatten()
            })
            .collect();
            if !due.is_empty() {
                context.with(|ctx| {
                    let Ok(fire) = ctx.globals().get::<_, Function>("__fire") else { return };
                    for id in due {
                        report(&ctx, "timer", fire.call::<_, ()>((id as f64,)), &*log);
                    }
                });
                continue;
            }
            mark(false);
            let wait = timers.borrow().peek().map_or(Duration::from_secs(3600), |Reverse((at, _, _))| at.saturating_duration_since(now));
            match receiver.recv_timeout(wait) {
                Ok(Inbox::Message(text)) => {
                    mark(true);
                    context.with(|ctx| {
                        let deliver: rquickjs::Result<Function> = ctx.globals().get("__deliver");
                        report(&ctx, "message", deliver.and_then(|f| f.call::<_, ()>((text,))), &*log);
                    });
                }
                Ok(Inbox::Stop) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    });
    Worker { sender, stopped }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::Receiver;

    fn start(source: &str) -> (Worker, Receiver<serde_json::Value>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let entry = dir.path().join("main.js");
        std::fs::write(&entry, source).unwrap();
        let (tx, rx) = channel();
        let logs = tx.clone();
        let worker = spawn("test".into(), entry.to_string_lossy().replace('\\', "/"), move |text| tx.send(serde_json::from_str(&text).unwrap()).unwrap(), move |level, message| {
            let _ = logs.send(serde_json::json!({ "type": "log", "level": level, "message": message }));
        });
        (worker, rx, dir)
    }

    fn next(rx: &Receiver<serde_json::Value>, kind: &str) -> serde_json::Value {
        loop {
            let message = rx.recv_timeout(Duration::from_secs(10)).expect("message");
            if message["type"] == kind {
                return message;
            }
            assert_ne!(message["type"], "log", "unexpected log: {message}");
        }
    }

    const FUNCTIONAL: &str = r#"
export default { handlers: {
  async search(query, ctx) {
    await new Promise((r) => setTimeout(r, 5));
    const u = new URL("https://example.com/a/b?x=1");
    u.searchParams.set("q", query);
    const stored = await ctx.native.call("storage.get", { key: "k" });
    return { type: "list", sections: [{ title: "S", items: [{ id: "1", title: u.href, subtitle: String(stored.value), keywords: [crypto.randomUUID().length.toString()] }] }] };
  },
  async onAction(id, item) { return { type: "detail", title: id + ":" + item.id, fields: [] }; },
} };
"#;

    #[test]
    fn runs_a_functional_extension_through_the_worker_protocol() {
        let (worker, rx, dir) = start(FUNCTIONAL);
        let url = dir.path().join("main.js").to_string_lossy().replace('\\', "/");
        let manifest = serde_json::json!({ "id": "demo", "name": "Demo", "version": "1.0.0", "commands": [{ "id": "c", "title": "C" }], "nativeMethods": [], "httpHosts": [] });
        worker.send(&serde_json::json!({ "type": "load", "requestId": "l1", "extensionId": "demo", "url": url, "manifest": manifest, "preferences": {}, "isDevelopment": false }));
        let loaded = next(&rx, "loaded");
        assert_eq!(loaded["ok"], true, "{loaded}");
        assert_eq!(loaded["kind"], "functional");
        worker.send(&serde_json::json!({ "type": "render", "viewId": "v1", "extensionId": "demo", "commandId": "c", "query": "hi there" }));
        let call = next(&rx, "nativeCall");
        assert_eq!(call["method"], "storage.get");
        worker.send(&serde_json::json!({ "type": "nativeResult", "requestId": call["requestId"], "ok": true, "result": { "ok": true, "value": 42 } }));
        let tree = next(&rx, "tree");
        let item = &tree["tree"]["sections"][0]["items"][0];
        assert_eq!(item["title"], "https://example.com/a/b?x=1&q=hi+there");
        assert_eq!(item["subtitle"], "42");
        assert_eq!(item["keywords"][0], "36");
        worker.send(&serde_json::json!({ "type": "action", "requestId": "a1", "viewId": "v1", "actionId": "open", "item": { "id": "1", "title": "x" } }));
        assert_eq!(next(&rx, "tree")["tree"]["title"], "open:1");
        assert_eq!(next(&rx, "done")["ok"], true);
    }

    #[test]
    fn renders_the_bundled_react_extension() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../extensions/spotify-player/dist");
        let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("manifest.json")).unwrap()).unwrap();
        let url = std::fs::canonicalize(root.join("main.js")).unwrap().to_string_lossy().trim_start_matches(r"\\?\").replace('\\', "/");
        let (tx, rx) = channel();
        let worker = spawn("spotify".into(), url.clone(), move |text| tx.send(serde_json::from_str::<serde_json::Value>(&text).unwrap()).unwrap(), |level, message| eprintln!("[{level}] {message}"));
        worker.send(&serde_json::json!({ "type": "load", "requestId": "l1", "extensionId": manifest["id"], "url": url, "manifest": manifest, "preferences": {}, "isDevelopment": false }));
        assert_eq!(next(&rx, "loaded")["kind"], "react");
        worker.send(&serde_json::json!({ "type": "render", "viewId": "v1", "extensionId": manifest["id"], "commandId": "search", "query": "" }));
        let tree = loop {
            let message = rx.recv_timeout(Duration::from_secs(10)).expect("message");
            match message["type"].as_str() {
                Some("nativeCall") => worker.send(&serde_json::json!({ "type": "nativeResult", "requestId": message["requestId"], "ok": false, "error": { "code": "notAuthorized", "message": "not signed in" } })),
                Some("tree") if message["tree"]["isLoading"] != true => break message["tree"].clone(),
                _ => {}
            }
        };
        assert!(tree["type"].is_string(), "{tree}");
    }
}
