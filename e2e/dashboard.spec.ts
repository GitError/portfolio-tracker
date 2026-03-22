import { test, expect } from '@playwright/test';

test.describe('Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(800);
  });

  test('displays portfolio value panel', async ({ page }) => {
    await expect(page.getByText('Portfolio Value', { exact: false }).first()).toBeVisible();
  });

  test('shows formatted currency value', async ({ page }) => {
    // At least one compact dollar value should appear
    const values = page.locator('text=/\\$[\\d.,]+[KMB]?/').first();
    await expect(values).toBeVisible();
  });

  test('shows top movers section', async ({ page }) => {
    await expect(page.getByText('Top Movers', { exact: false }).first()).toBeVisible();
  });

  test('displays total gain/loss', async ({ page }) => {
    await expect(page.getByText('Total Gain/Loss', { exact: false }).first()).toBeVisible();
  });

  test('shows allocation section', async ({ page }) => {
    await expect(page.getByText('Allocation', { exact: false }).first()).toBeVisible();
  });
});
