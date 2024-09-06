MDOJ is our two-person project, it unlikely to have entensive discussion with your,
but we welcome contribution.

> [!TIP]
> Contribution is not hard, and people(two-person) are nice, so don't be afraid.

# Priorities when making tradeoff

1. Reasonable performance
2. Ease of Use
3. Code size

# Pick an issue

At the time of writing, our short-term goal is to make it usable for hosting small contest.

Therefore, we prioritize [issue 16](https://github.com/mdcpp/mdoj/issues/16),
see issue label and pick issue with `P-High` or `good first issue`.

# Branching rule

We design branching rule

## Master branch

Branch suitable for development

- Without API change(see grpc crate), all branch should compile.
- Test can fail terribly.
- Breaking change should be created by pull request.
- Should format the code before push(clippy is not required).

## Staging branch

Branch suitable for nightly deployment

- deployment is ready(docker-compose can run)
- most test pass
- Clippy warning should be take care

> [!IMPORTANT]
> Please be sure API compatible before push master to staging

## Other branch

Related to issue (or just want to try).
