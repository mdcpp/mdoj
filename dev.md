# Versioning rule

## Master branch

Branch suitable for development

- application can run(standalone)
- test can fail terribly
- breaking change can only be push with pull request

## Staging branch

Branch suitable for nightly deployment

- deployment is ready(docker-compose can run)
- most test pass
- document(how to config) should be synchronized with wiki

## Other branch

Related to issue (or just want to try).
