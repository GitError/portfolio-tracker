import { test, expect } from '@playwright/test';

test.describe('Settings', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/settings');
  });

  test('renders settings page', async ({ page }) => {
    await expect(page.getByText('Settings').first()).toBeVisible();
  });

  test('shows Base Currency selector', async ({ page }) => {
    await expect(page.getByText('Base Currency').first()).toBeVisible();
    // CAD is the default
    await expect(page.getByText('CAD').first()).toBeVisible();
  });

  test('shows Auto-Refresh Interval setting', async ({ page }) => {
    await expect(page.getByText('Auto-Refresh Interval').first()).toBeVisible();
  });
});
