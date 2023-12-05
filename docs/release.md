# Release process

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

## Examples

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
