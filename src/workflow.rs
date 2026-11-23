//! The GitHub Actions workflow the Automation tab offers: tests first, and the deploy only if
//! they pass. An image app also builds and pushes its image to the registry its tag names. The
//! web panel's workflow.ts writes the same file.

use crate::api::App;

const TEST_JOB: &str = "  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # Your tests. For example: npm ci && npm test, go test ./..., pytest
      - run: make test";

fn deploy_step(app: &App, sha: bool) -> String {
    let sha = if sha {
        " \\\n            -d '{\"sha\": \"${{ github.sha }}\"}'"
    } else {
        ""
    };
    format!(
        "      - name: Deploy with walroos
        run: |
          curl -fsS -X POST \"$WALROOS_URL/hooks/deploy/{}\" \\
            -H \"Authorization: Bearer $WALROOS_DEPLOY_TOKEN\"{sha}
        env:
          WALROOS_URL: ${{{{ secrets.WALROOS_URL }}}}
          WALROOS_DEPLOY_TOKEN: ${{{{ secrets.WALROOS_DEPLOY_TOKEN }}}}",
        app.name
    )
}

pub fn github(app: &App) -> String {
    if app.image.is_empty() {
        let branch = if app.branch.is_empty() {
            "main"
        } else {
            &app.branch
        };
        return format!(
            "name: test and deploy
on:
  push:
    branches: [{branch}]

jobs:
{TEST_JOB}

  deploy:
    needs: test
    runs-on: ubuntu-latest
    steps:
{}
",
            deploy_step(app, true)
        );
    }
    let registry = app.image.split('/').next().unwrap_or_default();
    format!(
        "name: test, build and deploy
on:
  push:
    branches: [main]

jobs:
{TEST_JOB}

  deploy:
    needs: test
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4
      - uses: docker/login-action@v3
        with:
          registry: {registry}
          username: ${{{{ github.actor }}}}
          password: ${{{{ secrets.GITHUB_TOKEN }}}}
      - uses: docker/build-push-action@v6
        with:
          push: true
          tags: {}
{}
",
        app.image,
        deploy_step(app, false)
    )
}

#[cfg(test)]
mod tests {
    use super::github;
    use crate::api::fixtures;

    #[test]
    fn a_repo_app_deploys_the_commit_that_passed() {
        let w = github(&fixtures::app());
        assert!(w.contains("branches: [main]"), "{w}");
        assert!(w.contains("$WALROOS_URL/hooks/deploy/web"), "{w}");
        assert!(w.contains("-d '{\"sha\": \"${{ github.sha }}\"}'"), "{w}");
        assert!(
            w.contains("WALROOS_DEPLOY_TOKEN: ${{ secrets.WALROOS_DEPLOY_TOKEN }}"),
            "{w}"
        );
    }

    #[test]
    fn an_image_app_builds_and_pushes_it_first() {
        let app = crate::api::App {
            image: "ghcr.io/example/web:latest".into(),
            ..fixtures::app()
        };
        let w = github(&app);
        assert!(w.contains("registry: ghcr.io"), "{w}");
        assert!(w.contains("tags: ghcr.io/example/web:latest"), "{w}");
        assert!(!w.contains("github.sha"), "{w}");
    }
}
