# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in fallow, please report it responsibly via [GitHub's private vulnerability reporting](https://github.com/BartWaardenburg/fallow/security/advisories/new) instead of opening a public issue.

You should receive a response within 48 hours. Please include:

- A description of the vulnerability
- Steps to reproduce it
- Any relevant version or configuration information

## Scope

fallow is a static analysis tool that reads source files and `package.json`. It does not execute user code, make network requests, or modify files (except `fallow fix`, which only edits files in the analyzed project).
