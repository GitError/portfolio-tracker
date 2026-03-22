import { test, expect } from '@playwright/test';

test.describe('Stress Test', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/stress');
    await page.waitForTimeout(600);
  });

  test('shows Mild Correction selected by default', async ({ page }) => {
    await expect(page.getByText('Mild Correction').first()).toBeVisible();
  });

  test('auto-runs and shows results on load', async ({ page }) => {
    // Stress test auto-runs when the portfolio is available
    // Results table shows "Current Value" for the portfolio row
    await expect(page.getByText('Current Value').first()).toBeVisible();
  });

  test('shows stressed value column in results', async ({ page }) => {
    await expect(page.getByText(/Stressed Value/i).first()).toBeVisible();
  });

  test('selects Bear Market scenario via dropdown', async ({ page }) => {
    // Click the Select dropdown showing "Mild Correction" to open it
    await page.getByText('Mild Correction').first().click();
    // Options should appear
    await expect(page.getByText('Bear Market').first()).toBeVisible();
    await page.getByText('Bear Market').first().click();
    // Bear Market should now be selected
    await expect(page.getByText('Bear Market').first()).toBeVisible();
  });

  test('shows Compare Scenarios button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /compare scenarios/i })).toBeVisible();
  });
});
