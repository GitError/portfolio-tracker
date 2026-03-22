import { test, expect } from '@playwright/test';

// These tests run against the Vite dev server with mock data (no Tauri).

test.describe('Navigation', () => {
  test('loads the dashboard on root route', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(800);
    await expect(page).toHaveURL('/');
    await expect(page.getByText('Portfolio Value', { exact: false }).first()).toBeVisible();
  });

  test('navigates to Holdings page', async ({ page }) => {
    await page.goto('/holdings');
    await page.waitForTimeout(800);
    // Mock data has AAPL and MSFT
    await expect(page.getByText('AAPL').first()).toBeVisible();
  });

  test('navigates to Performance page', async ({ page }) => {
    await page.goto('/performance');
    await page.waitForTimeout(400);
    await expect(page.getByText('Performance', { exact: false }).first()).toBeVisible();
  });

  test('navigates to Stress Test page', async ({ page }) => {
    await page.goto('/stress');
    await page.waitForTimeout(400);
    // Stress test has a "Run Test" button and shows "Mild Correction" selected by default
    await expect(page.getByText('Mild Correction').first()).toBeVisible();
  });

  test('navigates to Settings page', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByText('Settings').first()).toBeVisible();
  });

  test('navigates to Rebalance page', async ({ page }) => {
    await page.goto('/rebalance');
    await page.waitForTimeout(800);
    await expect(page.getByText('Rebalance', { exact: false }).first()).toBeVisible();
  });

  test('navigates to Alerts page', async ({ page }) => {
    await page.goto('/alerts');
    await page.waitForTimeout(400);
    await expect(
      page.getByText('Alert', { exact: false }).first()
    ).toBeVisible();
  });

  test('navigates to Analytics page', async ({ page }) => {
    await page.goto('/analytics');
    await page.waitForTimeout(800);
    await expect(page.getByText('Analytics', { exact: false }).first()).toBeVisible();
  });
});
