# Security policy

## Reporting a vulnerability

Please do not open a public issue. Report privately through
[GitHub Security Advisories](https://github.com/taka10-web/envfish/security/advisories/new).

## Scope

EnvFish aims to stop an AI agent running with your own user account from reading
secret values. It cannot defend against a compromised OS, a root-level attacker,
or an agent you have granted unrestricted privileges.

The reasoning behind the design is in [docs/SECURITY-DESIGN.md](docs/SECURITY-DESIGN.md).
