# 099 Drop the stale cloudflare entry from infra/.terraform.lock.hcl

request: the coordinator, 2026-11-14 (cleanup after the move to ssh.lsh.tech)

`infra/versions.tf` no longer requires the cloudflare provider, but the lock file still pinned it. Regenerated with `tofu init -backend=false`: only the cloudflare block leaves. `tofu fmt -check` 0, `tofu validate` ok. No tofu plan or apply, no state touched.
