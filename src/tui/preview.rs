use crate::config::render::render;
use crate::config::schema::Config;
use crate::payload::Payload;
use crate::payload_mapping::build_render_ctx;

const FIXTURE_JSON: &str = include_str!("preview_fixture.json");

/// Load the embedded preview fixture once.
pub fn load_fixture() -> Payload {
    serde_json::from_str(FIXTURE_JSON).expect("preview_fixture.json must be valid JSON")
}

/// Render `config` against the stable preview fixture at `now_unix`.
///
/// Pure function: Config + Payload in, String out. No I/O, no dirty state.
pub fn render_preview(config: &Config, fixture: &Payload, now_unix: u64) -> String {
    let ctx = build_render_ctx(fixture, now_unix);
    render(config, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{Config, Line, Segment, TemplateSegment};

    fn fixture() -> Payload {
        load_fixture()
    }

    /// Stable now: far future so rate-limit countdowns are non-zero.
    const NOW: u64 = 1_700_000_000;

    #[test]
    fn preview_fixture_parses_as_valid_payload() {
        let p = fixture();
        assert!(p.model.is_some(), "fixture must have model");
        assert!(p.rate_limits.is_some(), "fixture must have rate_limits");
    }

    #[test]
    fn render_preview_against_default_config_produces_output() {
        let config = Config::default();
        let p = fixture();
        let out = render_preview(&config, &p, NOW);
        assert!(
            !out.is_empty(),
            "default config should produce non-empty output"
        );
    }

    #[test]
    fn render_preview_with_phase2_placeholders_resolves() {
        let config = Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: " | ".to_owned(),
                segments: vec![
                    // git_branch has no git repo in fixture → hide_when_absent drops it
                    Segment::Template(TemplateSegment {
                        template: "{git_branch}".to_owned(),
                        padding: 0,
                        hide_when_absent: true,
                        color: None,
                        bg: None,
                    }),
                    Segment::Template(TemplateSegment {
                        template: "{vim_mode}".to_owned(),
                        padding: 0,
                        hide_when_absent: false,
                        color: None,
                        bg: None,
                    }),
                    Segment::Template(TemplateSegment {
                        template: "${cost_usd}".to_owned(),
                        padding: 0,
                        hide_when_absent: false,
                        color: None,
                        bg: None,
                    }),
                ],
            }],
        };
        let p = fixture();
        let out = render_preview(&config, &p, NOW);
        // None of the placeholder tokens should appear literally in output.
        assert!(
            !out.contains("{git_branch}"),
            "git_branch must not render literally"
        );
        assert!(
            !out.contains("{vim_mode}"),
            "vim_mode must not render literally"
        );
        assert!(
            !out.contains("{cost_usd}"),
            "cost_usd must not render literally"
        );
    }

    #[test]
    fn render_preview_dirty_independent_of_render() {
        // render_preview is pure — dirty flag lives in App, not here.
        // Call it twice with the same inputs and verify identical results.
        let config = Config::default();
        let p = fixture();
        let a = render_preview(&config, &p, NOW);
        let b = render_preview(&config, &p, NOW);
        assert_eq!(a, b, "render_preview must be deterministic");
    }
}
