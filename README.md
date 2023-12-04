# slack-app

This component is a Slack app that is used to send Alerts to a Slack channel. It also can render a chart for a given alert.

## Release process

There is a GitHub action that allows us to publish a Docker image to our
internal AWS ECR registry and Docker Hub. There are two type of images that we
can publish: development images and production images.

The development images are intended for testing purposes. The Docker image tags
for these images follow the following format: `dev-<short sha>`. These images
are currently only published to our internal AWS ECR registry and will be purged
after a certain period of time.

The production images should be used by our users. The Docker image tags for
these images follow the following format: `v<version>`. These images are
published to both our internal AWS ECR registry and Docker Hub. These images
will never be purged.

For both images it is possible to override the `latest` tag.

To release a development images, leave the `version` input empty. Alternatively,
to release a production image, set a value to the `version` input (excluding the
`v` prefix).

It is both possible to trigger this process using the [GitHub web UI](https://github.com/autometrics/slack-app/actions/workflows/manual_build.yml)
(then select "run workflow") or using the [GitHub CLI](https://cli.github.com/).

### Examples

Release a development image based on a branch:

```
gh workflow run manual_build.yml \
    -f commitish=slack_app_ci_fixes
```

Release a production image based on a specific commit:

```
gh workflow run manual_build.yml \
    -f commitish=c0feee07f5cfc3d02339e42d8ecdb5eab9db3192 \
    -f version=1.0.0 \
    -f override_latest=true
```

### Known limitations

It is possible to deploy any version, even if these don't match the version of
the Rust app. It is up to the tooling/discretion of the release engineer to
ensure that these values match.

## How to Test Locally

Use quickmetrics instead of `am`, since it includes `alertmanager` out of the box, and it's configured to send alerts to the Slack app on localhost:3031.

```sh
git clone git@github.com:autometrics-dev/quickmetrics.git
cd quickmetrics
docker compose up --build
```

Then, set up ngrok to be able to receive requests from Slack to your local machine and generate images.

```sh
NGROK_DOMAIN=comic-dolphin-first.ngrok-free.app
ngrok http --domain=$NGROK_DOMAIN 3031
```

Then, launch the Slack app (get the bot token from our Slack dashboard)

```sh
STORAGE_DIR=/tmp \
  SLACK_CHANNEL=animalbuttons \
  SLACK_BOT_TOKEN=xoxb-123123 \
  PROMETHEUS_URL=http://localhost:9090 \
  EXPLORER_URL=http://localhost:6789 \
  BASE_URL=https://$NGROK_DOMAIN \
  RUST_LOG=info \
  cargo run
```

If you have an app running with quickmetrics that's generating alerts, then all this should work.

When I was testing, I used the python fastapi animals api, which should be running on port 8080. This will get scraped automagically by Prometheus.

```sh
git clone git@github.com:autometrics-dev/autometrics-demo-python-fastapi-animals.git
cd autometrics-demo-python-fastapi-animals
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
uvicorn app:app --reload --port=8080
```

There's a script in that repo for generating traffic. Alerts will start firing after a few minutes:

```sh
./generate-traffic.sh
```
