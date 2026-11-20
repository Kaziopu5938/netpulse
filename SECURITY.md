# Security Policy

## Supported versions

Only the latest released version of NetPulse receives security fixes.

| Version | Supported |
| --- | --- |
| 1.1.x | Yes |
| < 1.1 | No |

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Report vulnerabilities privately through GitHub's
[private vulnerability reporting](https://github.com/Kaziopu5938/netpulse/security/advisories/new)
on this repository. Include:

- a description of the issue and its impact,
- the affected version and operating system build,
- steps to reproduce, and
- a proof of concept if you have one.

You should receive an acknowledgement within **3 days** and a status update at
least every **7 days** until the issue is resolved. If a fix is released, we
will credit you in the release notes unless you prefer to remain anonymous.

## Scope

NetPulse is a local, per-user Windows utility. It reads local network counters,
writes a JSON config under `%APPDATA%\NetPulse`, creates an `HKCU\...\Run` entry
when autostart is enabled, and draws a click-through layered window in the
taskbar band. It does not open network listeners, handle remote input, or run
with elevated privileges.

Out of scope:

- vulnerabilities that require the attacker to already run code as the same
  user (that user can already change the config, the Run entry and the binary);
- issues in unsupported versions;
- Social-engineering / SmartScreen warnings caused by the absence of a
  code-signing certificate.

## Hardening

This repository protects `main` with branch protection and required CI checks
(build, tests, clippy, formatting, CodeQL). Dependencies are tracked by
Dependabot and `Cargo.lock` is committed so release builds are reproducible.
Release archives are accompanied by `SHA256SUMS.txt`.
