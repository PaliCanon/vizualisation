# No Worker and no DNS record are managed here any more: prod runs on
# ssh.lsh.tech behind the lsh-tech tunnel (Lsh0x/lsh-tech, stacks/palicanon/
# prod), which owns its record, and the dev Worker and its Custom Domain were
# removed (task 022). The `cloudflare` provider block stays in versions.tf for
# one more apply: the state still holds the dev Worker script and domain, and
# the provider is needed to destroy them. Remove the provider afterwards.
