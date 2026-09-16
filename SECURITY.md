# Security policy

## Reporting a vulnerability

Please do not open a public issue. Report privately through
[GitHub Security Advisories](https://github.com/taka10-web/envenb/security/advisories/new).

## Scope

EnvEnb aims to stop an AI agent running with your own user account from reading
secret values. It cannot defend against a compromised OS, a root-level attacker,
or an agent you have granted unrestricted privileges.

### What a session token can and cannot do

Applications reach providers through a local proxy, authenticated with a session
token that `envenb session` exports as `ENVENB_SESSION_TOKEN`. That token is in
the application's environment, so code running there — including code an agent
wrote — can read it and call the proxy.

This is deliberate: an application that may call an API needs *some* capability.
What it does not get is the credential itself, and the difference matters:

| | Provider credential | Session token |
|---|---|---|
| Lifetime | Until you rotate it | 12 hours by default |
| Reach | The whole account, from anywhere | Only the connections in its scope, only through a running proxy on this machine |
| Portable | Works on any machine | Useless without the local daemon |
| Visibility | Misuse leaves no trace in EnvEnb | Every call is in `envenb activity` |
| Revocation | Rotate at the provider | Stop the agent, or let it expire |

So a stolen token buys an attacker a bounded, logged, locally-scoped ability to
make calls you had already authorised — not a credential they can take away.
Narrow it further with `envenb session --connection <name> --ttl <seconds>`, and
set sensitive operations to `ASK` so they wait for you (`envenb ai permit`).

The reasoning behind the design is in [docs/SECURITY-DESIGN.md](docs/SECURITY-DESIGN.md).
