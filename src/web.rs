use crate::{
    app::AppService,
    domain::{DomainError, RunStatus},
    scenario::Scenario,
};
use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{
        Html, IntoResponse, Redirect, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use rust_embed::Embed;
use serde::Deserialize;
use std::{convert::Infallible, sync::Arc};

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

pub fn router(service: AppService) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/health", get(|| async { "ok" }))
        .route("/static/{*path}", get(static_asset))
        .route("/encounters/{scenario_id}", get(briefing))
        .route("/runs", post(create_run))
        .route("/runs/{run_id}/loadout", get(loadout).post(save_loadout))
        .route("/runs/{run_id}/start", post(start_run))
        .route("/runs/{run_id}", get(run_page))
        .route(
            "/runs/{run_id}/decisions/{decision_id}",
            post(resolve_decision),
        )
        .route("/runs/{run_id}/events", get(events))
        .route("/runs/{run_id}/postmortem", get(postmortem))
        .route("/runs/{run_id}/report.md", get(report))
        .with_state(Arc::new(service))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

async fn index() -> Result<Html<String>, WebError> {
    render(IndexTemplate)
}

#[derive(Template)]
#[template(path = "briefing.html")]
struct BriefingTemplate<'a> {
    scenario: &'a Scenario,
}

async fn briefing(
    State(app): State<Arc<AppService>>,
    Path(scenario_id): Path<String>,
) -> Result<Html<String>, WebError> {
    render(BriefingTemplate {
        scenario: app.scenario(&scenario_id)?,
    })
}

#[derive(Deserialize)]
struct CreateRunForm {
    scenario_id: String,
}

async fn create_run(
    State(app): State<Arc<AppService>>,
    Form(form): Form<CreateRunForm>,
) -> Result<Redirect, WebError> {
    let run = app.create_run(&form.scenario_id).await?;
    Ok(Redirect::to(&format!("/runs/{}/loadout", run.id)))
}

#[derive(Template)]
#[template(path = "loadout.html")]
struct LoadoutTemplate<'a> {
    scenario: &'a Scenario,
    run: crate::domain::RunState,
    error: Option<String>,
}

async fn loadout(
    State(app): State<Arc<AppService>>,
    Path(run_id): Path<String>,
) -> Result<Html<String>, WebError> {
    let run = app.load_run(&run_id).await?;
    let scenario = app.scenario(&run.scenario_id.0)?;
    render(LoadoutTemplate {
        scenario,
        run,
        error: None,
    })
}

#[derive(Deserialize)]
struct LoadoutForm {
    #[serde(default)]
    context: Vec<String>,
}

async fn save_loadout(
    State(app): State<Arc<AppService>>,
    Path(run_id): Path<String>,
    Form(form): Form<LoadoutForm>,
) -> Result<Response, WebError> {
    match app.configure(&run_id, form.context).await {
        Ok(run) => render(ReadyTemplate { run }).map(IntoResponse::into_response),
        Err(DomainError::InvalidLoadout(message)) => {
            let run = app.load_run(&run_id).await?;
            let scenario = app.scenario(&run.scenario_id.0)?;
            render(LoadoutTemplate {
                scenario,
                run,
                error: Some(message),
            })
            .map(IntoResponse::into_response)
        }
        Err(error) => Err(error.into()),
    }
}

#[derive(Template)]
#[template(path = "ready.html")]
struct ReadyTemplate {
    run: crate::domain::RunState,
}

async fn start_run(
    State(app): State<Arc<AppService>>,
    Path(run_id): Path<String>,
) -> Result<Redirect, WebError> {
    app.start(&run_id).await?;
    Ok(Redirect::to(&format!("/runs/{run_id}")))
}

#[derive(Template)]
#[template(path = "run.html")]
struct RunTemplate<'a> {
    scenario: &'a Scenario,
    run: crate::domain::RunState,
    events: Vec<crate::domain::TraceEvent>,
}

async fn run_page(
    State(app): State<Arc<AppService>>,
    Path(run_id): Path<String>,
) -> Result<Html<String>, WebError> {
    let run = app.load_run(&run_id).await?;
    let scenario = app.scenario(&run.scenario_id.0)?;
    let events = app.trace(&run_id, 0).await?;
    render(RunTemplate {
        scenario,
        run,
        events,
    })
}

#[derive(Deserialize)]
struct DecisionForm {
    prefer_current: bool,
}

async fn resolve_decision(
    State(app): State<Arc<AppService>>,
    Path((run_id, _decision_id)): Path<(String, String)>,
    Form(form): Form<DecisionForm>,
) -> Result<Redirect, WebError> {
    let run = app.decide(&run_id, form.prefer_current).await?;
    if matches!(run.status, RunStatus::Completed | RunStatus::Failed) {
        Ok(Redirect::to(&format!("/runs/{run_id}/postmortem")))
    } else {
        Ok(Redirect::to(&format!("/runs/{run_id}")))
    }
}

async fn events(
    State(app): State<Arc<AppService>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, WebError> {
    let after = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let trace = app.trace(&run_id, after).await?;
    let stream = async_stream::stream! {
        for item in trace {
            let event = Event::default()
                .event("trace")
                .id(item.sequence.to_string())
                .json_data(&item)
                .unwrap_or_else(|_| Event::default().event("error").data("serialization failed"));
            yield Ok::<_, Infallible>(event);
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[derive(Template)]
#[template(path = "postmortem.html")]
struct PostmortemTemplate<'a> {
    scenario: &'a Scenario,
    postmortem: crate::domain::Postmortem,
    run_id: String,
}

async fn postmortem(
    State(app): State<Arc<AppService>>,
    Path(run_id): Path<String>,
) -> Result<Html<String>, WebError> {
    let run = app.load_run(&run_id).await?;
    let scenario = app.scenario(&run.scenario_id.0)?;
    render(PostmortemTemplate {
        scenario,
        postmortem: app.postmortem(&run_id).await?,
        run_id,
    })
}

async fn report(
    State(app): State<Arc<AppService>>,
    Path(run_id): Path<String>,
) -> Result<Response, WebError> {
    let run = app.load_run(&run_id).await?;
    let scenario = app.scenario(&run.scenario_id.0)?;
    let postmortem = app.postmortem(&run_id).await?;
    let body = format!(
        "# Tracebound Expedition Report\n\n\
         Expedition: {}\n\nOutcome: {:?}\n\nScore: {}/100\n\n\
         Critical decision: {}\n\nLesson: {}\n",
        scenario.title,
        postmortem.outcome,
        postmortem.score,
        postmortem.critical_decision,
        postmortem.learning_principle
    );
    Ok((
        [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        body,
    )
        .into_response())
}

async fn static_asset(Path(path): Path<String>) -> Response {
    match StaticAssets::get(&path) {
        Some(asset) => (
            [(header::CONTENT_TYPE, content_type(&path))],
            asset.data.into_owned(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn render(template: impl Template) -> Result<Html<String>, WebError> {
    Ok(Html(template.render().map_err(WebError::template)?))
}

#[derive(Debug)]
struct WebError {
    status: StatusCode,
    message: String,
}

impl From<DomainError> for WebError {
    fn from(error: DomainError) -> Self {
        let status = match error {
            DomainError::ScenarioNotFound(_) | DomainError::RunNotFound(_) => StatusCode::NOT_FOUND,
            DomainError::InvalidTransition(_)
            | DomainError::InvalidLoadout(_)
            | DomainError::DecisionNotPending => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl WebError {
    fn template(error: askama::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        (
            self.status,
            Html(format!(
                "<main><h1>The expedition record resisted inspection.</h1>\
                 <p>{}</p><p><a href=\"/\">Return to safety</a></p></main>",
                escape_html(&self.message)
            )),
        )
            .into_response()
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::AppService, scenario::ScenarioRegistry, store::Store};
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    async fn test_router() -> Router {
        let store = Store::connect("sqlite::memory:")
            .await
            .expect("in-memory store");
        router(AppService {
            store,
            scenarios: ScenarioRegistry::load_embedded().expect("scenario"),
        })
    }

    #[tokio::test]
    async fn health_and_title_routes_succeed() {
        let app = test_router().await;
        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let title = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(title.status(), StatusCode::OK);
    }

    #[test]
    fn error_details_are_html_escaped() {
        assert_eq!(
            escape_html("<script>\"bad\" & 'worse'</script>"),
            "&lt;script&gt;&quot;bad&quot; &amp; &#39;worse&#39;&lt;/script&gt;"
        );
    }
}
