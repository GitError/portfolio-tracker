# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Shared portfolio state across views so holdings changes refresh dashboard, performance, and stress views automatically
- Target allocation support with rebalance deltas and deployable cash guidance
- CSV import with symbol validation and preview
- Account metadata for holdings, filters, and exports
- Configurable base currency across portfolio views

## [0.1.0] - 2026-03-14

### Added
- Dashboard with portfolio value, allocation charts, and top movers
- Holdings CRUD for stocks, ETFs, crypto, and cash
- Historical performance charts and portfolio stats
- Stress testing with preset and custom scenarios
- Live Yahoo Finance pricing and FX conversion
- Local SQLite persistence via Tauri and Rust
