import { test } from '@playwright/test';
test('debug', async ({ page }) => {
  page.on('console', msg => console.log('PAGE LOG:', msg.text()));
  page.on('pageerror', error => console.log('PAGE ERROR:', error.message));
  await page.goto('/');
  await page.waitForTimeout(2000);
});
