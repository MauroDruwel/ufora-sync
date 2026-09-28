<!--
  Mauro Quality Gate (MQG) — Canonical Pull Request Template
  Ensure all applicable checks are verified before requesting review.
-->

## 📌 Summary

<!-- Provide a concise description of what this PR introduces, fixes, or refactors. -->

### Rationale & Context
<!-- Why is this change necessary? What problem does it solve? -->

---

## 🛠️ Type of Change

- [ ] 🐛 **Bug fix** (non-breaking fix for an unexpected issue)
- [ ] ✨ **New feature** (non-breaking addition of functionality)
- [ ] ♻️ **Refactoring** (structural code improvements without behavioral changes)
- [ ] ⚡ **Performance improvement** (optimizations reducing latency or resource usage)
- [ ] 📝 **Documentation** (updates to README, architecture guides, or docstrings)
- [ ] 🔧 **CI/CD & Tooling** (workflow updates, linters, packaging)

---

## 🏛️ Mauro Quality Gate Checklist

Please verify compliance with the **6 Pillars of Mauro Quality Gate**:

- [ ] **1. Governance & Hygiene**: No extraneous temporary files (`.DS_Store`, build caches) included; clean git history.
- [ ] **2. Architecture**: Follows canonical repository patterns, committed lockfiles, and zero deprecated dependencies.
- [ ] **3. CI/CD Ready**: Passes all automated checks and matrix builds on GitHub Actions without warnings.
- [ ] **4. Type Safety & Strictness**: Zero lint warnings (`ruff` / `eslint` / `cargo clippy -D warnings`), strict type checks.
- [ ] **5. Testing & Verification**: Unit / integration tests added or updated; 100% of test suite passes locally.
- [ ] **6. Changelog & SemVer**: `CHANGELOG.md` updated according to Keep a Changelog if applicable.

---

## 🧪 Verification & Evidence

<!-- Paste relevant command outputs, terminal logs, or before/after screenshots demonstrating the fix or feature. -->

```bash
# Example test run output:
```
