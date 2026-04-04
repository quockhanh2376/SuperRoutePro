# Frontend Performance Baseline

Captured on `2026-04-04` with:

```bash
npm run profile:frontend
```

Current synthetic baseline:

- `buildIpScanPlan`: `143.89 ms` total / `5000` iterations / `0.0288 ms` average
- `mergeNicDescriptions`: `253.95 ms` total / `5000` iterations / `0.0508 ms` average
- `formatRoutingSnapshot`: `240.45 ms` total / `1000` iterations / `0.2404 ms` average
- `validateRouteForm`: `20.58 ms` total / `10000` iterations / `0.0021 ms` average

What this covers:

- route-form validation hot path
- NIC description enrichment
- routing snapshot formatting
- subnet scan plan construction

Notes:

- These numbers are synthetic Node-side baselines, useful for regression checks after refactors.
- UI paint cost, React reconciliation, and Tauri round-trip latency still need browser/native profiling when investigating user-visible slowness.
