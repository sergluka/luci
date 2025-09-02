use luci::execution::{Executable, SourceCodeLoader};
use luci::marshalling::{MarshallingRegistry, Regular, Request};
use serde_json::json;

pub mod proto {
    use elfo::message;
    use serde_json::Value;

    #[message]
    pub struct Hey;

    #[message]
    pub struct V(pub Value);

    #[message(ret = Value)]
    pub struct R(pub Value);
}

pub mod echo {
    use elfo::{msg, ActorGroup, Blueprint, Context};
    use serde_json::json;

    use crate::proto;

    pub async fn actor(mut ctx: Context) {
        while let Some(envelope) = ctx.recv().await {
            let sender = envelope.sender();
            msg!(match envelope {
                proto::Hey => {
                    ctx.request_to(sender, proto::R(json!("hello!")))
                        .resolve()
                        .await
                        .expect("oh :(");
                },
                v @ proto::V => {
                    let _ = ctx.send_to(sender, v).await;
                },
                (r @ proto::R, t) => {
                    let _ = ctx.respond(t, r.0);
                },
            })
        }
    }

    pub fn blueprint() -> Blueprint {
        ActorGroup::new().exec(actor)
    }
}

#[tokio::test]
async fn bind_node() {
    run_scenario("tests/echo/bind-node.luci.yaml", []).await;
}

#[tokio::test]
async fn marshalling() {
    run_scenario("tests/echo/marshalling.luci.yaml", []).await;
}

#[tokio::test]
async fn request_response() {
    run_scenario("tests/echo/request-response.luci.yaml", []).await;
}

#[tokio::test]
async fn check_init_bind() {
    run_scenario(
        "tests/echo/check-init-bind.luci.yaml",
        [
            ("$ARG_1".into(), json!("one")),
            ("$ARG_2".into(), json!("two")),
        ],
    )
    .await;
}

async fn run_scenario(
    scenario_file: &str,
    args: impl IntoIterator<Item = (String, serde_json::Value)>,
) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_max_level(tracing::Level::TRACE)
        .try_init();
    tokio::time::pause();

    let marshalling = MarshallingRegistry::new()
        .with(Regular::<crate::proto::V>)
        .with(Request::<crate::proto::R>)
        .with(Regular::<crate::proto::Hey>);

    let (key_main, sources) = SourceCodeLoader::new()
        .load(scenario_file)
        .expect("SourceLoader::load");
    let executable = Executable::build(marshalling, &sources, key_main).expect("building graph");
    let report = executable
        .start(echo::blueprint(), json!(null), args)
        .await
        .run()
        .await
        .expect("runner.run");

    let _ = report.dump_record_log(std::io::stderr().lock(), &sources, &executable);
    assert!(report.is_ok(), "{}", report.message());
}
