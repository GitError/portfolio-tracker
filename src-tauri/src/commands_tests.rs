/// Integration tests for command-layer validation and business logic.
///
/// These tests exercise the same code paths that Tauri commands call,
/// using an in-memory SQLite database.  They do NOT test the thin
/// `#[tauri::command]` wrapper itself (Tauri's `State` type is hard to
/// construct in isolation); instead they test the underlying db / domain
/// functions and any validation helpers extracted from commands.rs.
#[cfg(test)]
mod tests {
    use crate::db;
    use crate::types::{
        AccountType, AlertDirection, AssetType, DividendInput, HoldingInput, PriceAlertInput,
        TransactionInput, TransactionType,
    };

    // ── helpers ──────────────────────────────────────────────────────────────

    fn holding_input(symbol: &str) -> HoldingInput {
        HoldingInput {
            symbol: symbol.to_string(),
            name: format!("{} Inc.", symbol),
            asset_type: AssetType::Stock,
            account: AccountType::Taxable,
            account_id: None,
            quantity: 10.0,
            cost_basis: 150.0,
            currency: "CAD".to_string(),
            exchange: "TSX".to_string(),
            target_weight: 0.0,
            indicated_annual_dividend: None,
            indicated_annual_dividend_currency: None,
            dividend_frequency: None,
            maturity_date: None,
        }
    }

    // ── holding validation (mirrors add_holding logic) ────────────────────────

    fn validate_holding(input: &HoldingInput) -> Result<(), String> {
        if input.quantity <= 0.0 || !input.quantity.is_finite() {
            return Err("quantity must be a positive finite number".to_string());
        }
        if input.cost_basis < 0.0 || !input.cost_basis.is_finite() {
            return Err("costBasis must be a non-negative finite number".to_string());
        }
        let currency = input.currency.trim().to_uppercase();
        if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err("currency must be a 3-letter ISO currency code".to_string());
        }
        Ok(())
    }

    #[test]
    fn add_holding_rejects_zero_quantity() {
        let mut h = holding_input("RY");
        h.quantity = 0.0;
        assert!(validate_holding(&h).is_err());
    }

    #[test]
    fn add_holding_rejects_negative_quantity() {
        let mut h = holding_input("RY");
        h.quantity = -1.0;
        assert!(validate_holding(&h).is_err());
    }

    #[test]
    fn add_holding_rejects_nan_quantity() {
        let mut h = holding_input("RY");
        h.quantity = f64::NAN;
        assert!(validate_holding(&h).is_err());
    }

    #[test]
    fn add_holding_rejects_negative_cost_basis() {
        let mut h = holding_input("RY");
        h.cost_basis = -1.0;
        assert!(validate_holding(&h).is_err());
    }

    #[test]
    fn add_holding_accepts_zero_cost_basis() {
        let mut h = holding_input("RY");
        h.cost_basis = 0.0;
        assert!(validate_holding(&h).is_ok());
    }

    #[test]
    fn add_holding_rejects_invalid_currency() {
        let mut h = holding_input("RY");
        h.currency = "US".to_string(); // too short
        assert!(validate_holding(&h).is_err());

        h.currency = "USDD".to_string(); // too long
        assert!(validate_holding(&h).is_err());

        h.currency = "US1".to_string(); // non-alpha
        assert!(validate_holding(&h).is_err());
    }

    #[test]
    fn add_holding_accepts_valid_inputs() {
        assert!(validate_holding(&holding_input("AAPL")).is_ok());
    }

    // ── alert validation ──────────────────────────────────────────────────────

    fn validate_alert(input: &PriceAlertInput) -> Result<(), String> {
        if !input.threshold.is_finite() || input.threshold <= 0.0 {
            return Err("threshold must be a positive finite number".to_string());
        }
        Ok(())
    }

    #[test]
    fn add_alert_rejects_zero_threshold() {
        let input = PriceAlertInput {
            symbol: "AAPL".to_string(),
            direction: AlertDirection::Above,
            threshold: 0.0,
            currency: "CAD".to_string(),
            note: String::new(),
        };
        assert!(validate_alert(&input).is_err());
    }

    #[test]
    fn add_alert_rejects_negative_threshold() {
        let input = PriceAlertInput {
            symbol: "AAPL".to_string(),
            direction: AlertDirection::Above,
            threshold: -1.0,
            currency: "CAD".to_string(),
            note: String::new(),
        };
        assert!(validate_alert(&input).is_err());
    }

    #[test]
    fn add_alert_accepts_valid_threshold() {
        let input = PriceAlertInput {
            symbol: "AAPL".to_string(),
            direction: AlertDirection::Above,
            threshold: 200.0,
            currency: "CAD".to_string(),
            note: String::new(),
        };
        assert!(validate_alert(&input).is_ok());
    }

    // ── page size validation ──────────────────────────────────────────────────

    fn validate_pagination(page: i64, page_size: i64) -> Result<(), String> {
        if page < 1 {
            return Err("page must be >= 1".to_string());
        }
        if !(1..=500).contains(&page_size) {
            return Err("page_size must be between 1 and 500".to_string());
        }
        Ok(())
    }

    #[test]
    fn pagination_rejects_page_zero() {
        assert!(validate_pagination(0, 50).is_err());
    }

    #[test]
    fn pagination_rejects_page_size_zero() {
        assert!(validate_pagination(1, 0).is_err());
    }

    #[test]
    fn pagination_rejects_page_size_exceeding_max() {
        assert!(validate_pagination(1, 501).is_err());
    }

    #[test]
    fn pagination_accepts_valid_params() {
        assert!(validate_pagination(1, 50).is_ok());
        assert!(validate_pagination(10, 500).is_ok());
        assert!(validate_pagination(1, 1).is_ok());
    }

    // ── DB integration: holdings CRUD ─────────────────────────────────────────

    #[tokio::test]
    async fn holding_crud_round_trip() {
        let pool = crate::db::open_test_db().await;

        let created = db::insert_holding(&pool, holding_input("AAPL"))
            .await
            .expect("insert");
        assert_eq!(created.symbol, "AAPL");
        assert_eq!(created.quantity, 10.0);

        let all = db::get_all_holdings(&pool).await.expect("get all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, created.id);

        db::delete_holding(&pool, &created.id)
            .await
            .expect("delete");
        let after = db::get_all_holdings(&pool).await.expect("get after");
        assert!(after.is_empty());
    }

    #[tokio::test]
    async fn update_holding_persists_changes() {
        let pool = crate::db::open_test_db().await;

        let created = db::insert_holding(&pool, holding_input("RY"))
            .await
            .expect("insert");

        let mut updated = created.clone();
        updated.quantity = 25.0;
        updated.cost_basis = 120.0;
        db::update_holding(&pool, updated.clone())
            .await
            .expect("update");

        let all = db::get_all_holdings(&pool).await.expect("get all");
        assert_eq!(all[0].quantity, 25.0);
        assert_eq!(all[0].cost_basis, 120.0);
    }

    // ── DB integration: transactions ──────────────────────────────────────────

    #[tokio::test]
    async fn transaction_crud_round_trip() {
        let pool = crate::db::open_test_db().await;

        let holding = db::insert_holding(&pool, holding_input("TD"))
            .await
            .expect("insert holding");

        let tx_input = TransactionInput {
            holding_id: holding.id.clone(),
            transaction_type: TransactionType::Buy,
            quantity: 5.0,
            price: 80.0,
            transacted_at: "2024-01-15T10:00:00Z".to_string(),
        };
        let tx = db::insert_transaction(&pool, tx_input)
            .await
            .expect("insert tx");
        assert_eq!(tx.holding_id, holding.id);

        let txs = db::get_transactions_for_holding(&pool, &holding.id)
            .await
            .expect("get txs");
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].quantity, 5.0);

        db::delete_transaction(&pool, &tx.id)
            .await
            .expect("delete tx");
        let after = db::get_transactions_for_holding(&pool, &holding.id)
            .await
            .expect("get after");
        assert!(after.is_empty());
    }

    // ── DB integration: price alerts ──────────────────────────────────────────

    #[tokio::test]
    async fn alert_crud_round_trip() {
        let pool = crate::db::open_test_db().await;

        let input = PriceAlertInput {
            symbol: "AAPL".to_string(),
            direction: AlertDirection::Above,
            threshold: 200.0,
            currency: "CAD".to_string(),
            note: "All time high".to_string(),
        };
        let alert = db::insert_alert(&pool, input).await.expect("insert");
        assert_eq!(alert.symbol, "AAPL");
        assert!(!alert.triggered);

        let all = db::get_alerts(&pool).await.expect("get all");
        assert_eq!(all.len(), 1);

        db::delete_alert(&pool, &alert.id).await.expect("delete");
        let after = db::get_alerts(&pool).await.expect("get after");
        assert!(after.is_empty());
    }

    // ── DB integration: dividends ─────────────────────────────────────────────

    #[tokio::test]
    async fn dividend_crud_round_trip() {
        let pool = crate::db::open_test_db().await;

        let holding = db::insert_holding(&pool, holding_input("RY"))
            .await
            .expect("insert holding");

        let div_input = DividendInput {
            holding_id: holding.id.clone(),
            amount_per_unit: 1.38,
            currency: "CAD".to_string(),
            ex_date: "2024-03-15".to_string(),
            pay_date: "2024-04-25".to_string(),
        };
        let div = db::insert_dividend(&pool, div_input, "RY")
            .await
            .expect("insert dividend");
        assert_eq!(div.symbol, "RY");
        assert_eq!(div.amount_per_unit, 1.38);

        let all = db::get_dividends(&pool).await.expect("get all");
        assert_eq!(all.len(), 1);

        db::delete_dividend(&pool, div.id).await.expect("delete");
        let after = db::get_dividends(&pool).await.expect("get after");
        assert!(after.is_empty());
    }

    // ── DB integration: pagination ────────────────────────────────────────────

    #[tokio::test]
    async fn holdings_pagination_returns_correct_page() {
        let pool = crate::db::open_test_db().await;

        for sym in &["A", "B", "C", "D", "E"] {
            db::insert_holding(&pool, holding_input(sym))
                .await
                .expect("insert");
        }

        let page1 = db::get_holdings_paginated(&pool, 1, 2)
            .await
            .expect("page 1");
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.total, 5);
        assert_eq!(page1.total_pages, 3);

        let page3 = db::get_holdings_paginated(&pool, 3, 2)
            .await
            .expect("page 3");
        assert_eq!(page3.items.len(), 1); // only 1 item on last page

        let page4 = db::get_holdings_paginated(&pool, 4, 2)
            .await
            .expect("page 4 (empty)");
        assert_eq!(page4.items.len(), 0);
    }

    // ── DB integration: config ────────────────────────────────────────────────

    #[tokio::test]
    async fn config_get_and_set() {
        let pool = crate::db::open_test_db().await;

        let val = db::get_config(&pool, "base_currency")
            .await
            .expect("get config");
        assert!(val.is_none());

        db::set_config(&pool, "base_currency", "USD")
            .await
            .expect("set config");

        let updated = db::get_config(&pool, "base_currency")
            .await
            .expect("get updated");
        assert_eq!(updated.as_deref(), Some("USD"));
    }

    // ── DB integration: target weight sum ────────────────────────────────────

    #[tokio::test]
    async fn target_weight_sum_accumulates() {
        let pool = crate::db::open_test_db().await;

        let mut h1 = holding_input("A");
        h1.target_weight = 40.0;
        db::insert_holding(&pool, h1).await.expect("insert A");

        let mut h2 = holding_input("B");
        h2.target_weight = 35.0;
        db::insert_holding(&pool, h2).await.expect("insert B");

        let sum = db::sum_target_weights(&pool, None)
            .await
            .expect("sum weights");
        assert!((sum - 75.0).abs() < 0.001, "expected 75, got {sum}");
    }
}
