use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    AbortController, Document, Element, Event, HtmlInputElement, Request, RequestInit, Response,
};

const DEBOUNCE_MS: i32 = 180;
const READ_TIMEOUT_MS: i32 = 30_000;
struct Search {
    document: Document,
    input: HtmlInputElement,
    region: Element,
    revision: Option<String>,
    generation: super::Generation,
    composing: bool,
    timer: Option<i32>,
    timer_callback: Option<Closure<dyn FnMut()>>,
    active: Option<AbortController>,
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let document = window()
        .document()
        .ok_or_else(|| JsValue::from_str("document absent"))?;
    let input = document
        .get_element_by_id("query")
        .ok_or_else(|| JsValue::from_str("query absent"))?
        .dyn_into::<HtmlInputElement>()?;
    let region = document
        .get_element_by_id("search-region")
        .ok_or_else(|| JsValue::from_str("results absent"))?;
    let form = document
        .query_selector("form")?
        .ok_or_else(|| JsValue::from_str("form absent"))?;
    let revision = document
        .query_selector("input[name=revision]")?
        .map(|node| node.dyn_into::<HtmlInputElement>())
        .transpose()?
        .map(|node| node.value());
    let search = Rc::new(RefCell::new(Search {
        document,
        input: input.clone(),
        region,
        revision,
        generation: super::Generation::default(),
        composing: false,
        timer: None,
        timer_callback: None,
        active: None,
    }));
    listen(input.as_ref(), "input", {
        let search = search.clone();
        move |_| {
            change(&search, false);
        }
    })?;
    listen(input.as_ref(), "compositionstart", {
        let search = search.clone();
        move |_| {
            let mut state = search.borrow_mut();
            state.composing = true;
            state.invalidate();
            state.message("composing", "Composing search…");
        }
    })?;
    listen(input.as_ref(), "compositionend", {
        let search = search.clone();
        move |_| {
            search.borrow_mut().composing = false;
            change(&search, false);
        }
    })?;
    listen(form.as_ref(), "submit", {
        let search = search.clone();
        move |event| {
            event.prevent_default();
            if !search.borrow().composing {
                change(&search, true);
            }
        }
    })?;
    input.set_attribute("data-live-search", "ready")?;
    Ok(())
}
fn window() -> web_sys::Window {
    web_sys::window().expect("browser window")
}
fn listen(
    target: &web_sys::EventTarget,
    name: &str,
    callback: impl FnMut(Event) + 'static,
) -> Result<(), JsValue> {
    let callback = Closure::<dyn FnMut(Event)>::new(callback);
    target.add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())?;
    callback.forget(); // One listener set for the document lifetime.
    Ok(())
}
impl Search {
    fn invalidate(&mut self) -> u64 {
        if let Some(timer) = self.timer.take() {
            window().clear_timeout_with_handle(timer);
        }
        self.timer_callback = None;
        if let Some(active) = self.active.take() {
            active.abort();
        }
        self.generation.next()
    }
    fn message(&self, state: &str, message: &str) {
        self.region.set_text_content(Some(message));
        let _ = self.region.set_attribute("data-search-state", state);
        let _ = self.region.set_attribute(
            "aria-busy",
            if state == "loading" { "true" } else { "false" },
        );
    }
}
fn change(search: &Rc<RefCell<Search>>, immediate: bool) {
    let mut state = search.borrow_mut();
    let generation = state.invalidate();
    let query = state.input.value();
    if state.composing {
        return;
    }
    if query.trim().is_empty() {
        state.message("empty", "Start with a name.");
        return;
    }
    state.message("loading", "Searching…");
    if immediate {
        drop(state);
        begin(search.clone(), generation, query);
        return;
    }
    let weak = Rc::downgrade(search);
    let callback = Closure::<dyn FnMut()>::new(move || {
        if let Some(search) = weak.upgrade() {
            begin(search, generation, query.clone());
        }
    });
    match window().set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        DEBOUNCE_MS,
    ) {
        Ok(timer) => {
            state.timer = Some(timer);
            state.timer_callback = Some(callback);
        }
        Err(_) => state.message(
            "failed",
            "Search is unavailable. Press Search to try again.",
        ),
    }
}
fn begin(search: Rc<RefCell<Search>>, generation: u64, query: String) {
    let mut state = search.borrow_mut();
    if !state.generation.accepts(generation) {
        return;
    }
    state.timer = None;
    let Ok(controller) = AbortController::new() else {
        state.message(
            "failed",
            "Search is unavailable. Press Search to try again.",
        );
        return;
    };
    state.active = Some(controller.clone());
    let revision = state.revision.clone();
    drop(state);
    spawn_local(async move {
        let timeout_controller = controller.clone();
        let timeout = Closure::<dyn FnMut()>::new(move || timeout_controller.abort());
        let timer = window().set_timeout_with_callback_and_timeout_and_arguments_0(
            timeout.as_ref().unchecked_ref(),
            READ_TIMEOUT_MS,
        );
        if timer.is_err() {
            controller.abort();
        }
        let result = read(&query, revision.as_deref(), &controller).await;
        if let Ok(timer) = timer {
            window().clear_timeout_with_handle(timer);
        }
        let mut state = search.borrow_mut();
        if !state.generation.accepts(generation) {
            return;
        }
        state.active = None;
        match result {
            Ok(region) => {
                // Import just the native renderer's result/revision region, never the input or
                // form. Parsing is inert; only same-origin server HTML reaches this boundary.
                match state.document.import_node_with_deep(&region, true) {
                    Ok(imported) => {
                        let imported: Element = imported.unchecked_into();
                        let _ = imported.set_attribute("data-search-state", "ready");
                        let _ = imported.set_attribute("aria-busy", "false");
                        if state.region.replace_with_with_node_1(&imported).is_ok() {
                            state.region = imported;
                        } else {
                            state.message(
                                "failed",
                                "Search is unavailable. Press Search to try again.",
                            );
                        }
                    }
                    Err(_) => state.message(
                        "failed",
                        "Search is unavailable. Press Search to try again.",
                    ),
                }
            }
            Err(_) => state.message(
                "failed",
                "Search is unavailable. Press Search to try again.",
            ),
        }
    });
}
async fn read(
    query: &str,
    revision: Option<&str>,
    controller: &AbortController,
) -> Result<Element, JsValue> {
    let params = web_sys::UrlSearchParams::new()?;
    params.append("q", query);
    if let Some(revision) = revision {
        params.append("revision", revision);
    }
    let init = RequestInit::new();
    init.set_signal(Some(&controller.signal()));
    init.set_mode(web_sys::RequestMode::SameOrigin);
    init.set_credentials(web_sys::RequestCredentials::SameOrigin);
    let request = Request::new_with_str_and_init(&format!("/find?{}", params.to_string()), &init)?;
    let response = JsFuture::from(window().fetch_with_request(&request))
        .await?
        .dyn_into::<Response>()?;
    // 404 is the renderer's honest unavailable-revision page. Other failures clear the region.
    if !response.ok() && response.status() != 404 {
        return Err(JsValue::from_str("search refused"));
    }
    let html = JsFuture::from(response.text()?)
        .await?
        .as_string()
        .ok_or_else(|| JsValue::from_str("nontext reply"))?;
    let document =
        web_sys::DomParser::new()?.parse_from_string(&html, web_sys::SupportedType::TextHtml)?;
    document
        .get_element_by_id("search-region")
        .ok_or_else(|| JsValue::from_str("missing result region"))
}
