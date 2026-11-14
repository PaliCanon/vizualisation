# 024 GET /healthz

status: in review
rank: 1
level: simple
request: the coordinator, 2026-11-14: one uniform `GET /healthz` on every public service, for external uptime checks

## Summary

`GET /healthz` answers 200, the body `ok`, `Cache-Control: no-store`, behind the origin guard only (no password, no app render). `/health` stays as an alias: the server's probes, the rollout check and the smoke test still read it. The Cloud Run comment and the test that kept `/health` clear of the `z` paths are gone (Cloud Run left, task 023).

## Log

| Step | Result |
| --- | --- |
| Red | `cargo test --locked -p vizualisation --features ssr --lib router`: `healthz_answers_ok_uncached_without_the_password` and `the_layers_answer_in_the_designs_order` FAILED, exit 101 |
| Green | same command: 14 passed, exit 0 |
| Checks | `cargo fmt --all --check`, `leptosfmt --check app`, clippy ssr `--all-targets -D warnings`: exit 0 |
