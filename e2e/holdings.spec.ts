import { test, expect } from '@playwright/test';

test.describe('Holdings', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/holdings');
    await page.waitForTimeout(800);
  });

  test('shows mock holdings in table', async ({ page }) => {
    await expect(page.getByRole('cell', { name: 'AAPL' })).toBeVisible();
    await expect(page.getByRole('cell', { name: 'MSFT' })).toBeVisible();
  });

  test('shows Add Holding button', async ({ page }) => {
    await expect(page.getByRole('button', { name: 'Add Holding' })).toBeVisible();
  });

  test('opens Add Holding modal', async ({ page }) => {
    await page.getByRole('button', { name: 'Add Holding' }).click();
    await expect(page.getByRole('heading', { name: 'Add Holding' })).toBeVisible();
    await expect(page.getByPlaceholder(/symbol/i)).toBeVisible();
  });

  test('closes Add Holding modal via Cancel', async ({ page }) => {
    await page.getByRole('button', { name: 'Add Holding' }).click();
    await expect(page.getByRole('heading', { name: 'Add Holding' })).toBeVisible();

    await page.getByRole('button', { name: 'Cancel' }).click();
    await expect(page.getByRole('heading', { name: 'Add Holding' })).not.toBeVisible();
  });

  test('shows Import CSV button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /import/i })).toBeVisible();
  });

  test('shows Export CSV button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /export/i })).toBeVisible();
  });

  test('shows table column headers', async ({ page }) => {
    await expect(page.getByRole('columnheader', { name: /symbol/i })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: /quantity/i })).toBeVisible();
  });
});
