# Test fixtures

Shared test inputs for Cipherglass. F-001 defines the policy; later features
add the fixtures.

## Policy

- **Synthetic or public only.** Fixtures are written for this repository, or
  are public files under a license compatible with Apache-2.0 (record the
  source and license next to the fixture).
- **No client-derived material, ever.** Do not copy, trim, or "anonymize"
  JavaScript from a client engagement. This repository is public.
- **No real credentials**, including revoked or expired ones. Use obviously
  fake values.
- **Inert data.** Fixtures are read, never executed (REQ-023). Hostile and
  malformed samples are welcome; that is what Cipherglass must survive.

## Layout

```
tests/fixtures/<area>/<case>/
    <input files>
    expected/        optional: expected output for this case
```

`<area>` matches a feature spec `area` value (e.g. `discovery`,
`parser-adapter`, `analyzer`). `<case>` is a short kebab-case description,
such as `minified-apollo-bundle` or `truncated-template-literal`.
