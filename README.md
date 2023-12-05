# slack-app

This component is a Slack app that is used to send Alerts to a Slack channel. It
also can render a chart for a given alert.

## How to Test Locally

To be able to test the slack-app locally you will need to expose a http service
to the API servers of Slack, in this example we're using (`ngrok`)[https://ngrok.com/]
to do so. We're also using `quickmetrics` instead of `am` since that includes
`alertmanager`. We'll start by first retrieving quickmetrics and starting that
using `docker-compose`.

```sh
git clone git@github.com:autometrics-dev/quickmetrics.git
cd quickmetrics
docker compose up --build
```

Next, we will set up ngrok to be able to receive requests from Slack to your
local machine and retrieve the graphs (note: update the domain to your own
unique domain):

```sh
NGROK_DOMAIN=comic-dolphin-first.ngrok-free.app
ngrok http --domain=$NGROK_DOMAIN 3031
```

You will need to create and install a Slack app in your Slack workspace. See the
instructions in the [autometrics docs](http://docs.autometrics.dev/deploying-alertmanager/kubernetes).
Then we will start the slack-app on your local machine (note: be sure to update
the `SLACK_BOT_TOKEN` and `SLACK_CHANNEL`):

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

If you have an app running with quickmetrics that's generating alerts, then all
this should work.

To be able to quickly trigger a alert you can also use the following sample app:

```sh
git clone git@github.com:autometrics-dev/autometrics-demo-python-fastapi-animals.git
cd autometrics-demo-python-fastapi-animals
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
uvicorn app:app --reload --port=8080
```

Once that is running, it should be scraped by quickmetrics and if you run the
traffic generate script it should fire alerts in a couple of minutes:

```sh
./generate-traffic.sh
```
