import { test, expect } from '@playwright/test';

test.describe('CodeGraph E2E', () => {
  test('should load the graph and display mock data', async ({ page }) => {
    await page.goto('/');

    // Verify mock data is loaded
    const repoPath = page.locator('.eyeloss-controls__repo');
    await expect(repoPath).toHaveText('/path/to/despair');

    // Wait for spinner to disappear
    await expect(page.locator('.eyeloss-spinner')).not.toBeVisible();

    // Check if the canvas is there
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();

    // Check the nav panel for the mock node (now shows filenames)
    const navPanel = page.locator('.eyeloss-nav-panel');
    await expect(navPanel).toContainText('MockModule');
  });

  test('should allow changing the repository path', async ({ page }) => {
    await page.goto('/');

    // Click "Change" button (the mock returns /new/path/to/despair)
    const changeBtn = page.locator('button', { hasText: 'Change' });
    await changeBtn.click();

    // Verify path changed
    const repoPath = page.locator('.eyeloss-controls__repo');
    await expect(repoPath).toHaveText('/new/path/to/despair');
  });

  test('should toggle the nav panel', async ({ page }) => {
    await page.goto('/');
    
    const toggleBtn = page.locator('.eyeloss-nav-panel__toggle');
    await expect(page.locator('.eyeloss-nav-panel__search')).toBeVisible();
    
    await toggleBtn.click();
    await expect(page.locator('.eyeloss-nav-panel__search')).not.toBeVisible();
  });

  test('should display and interact with the timeline', async ({ page }) => {
    await page.goto('/');

    // Wait for data
    await expect(page.locator('.eyeloss-timeline')).toBeVisible();

    // Timeline window is dynamic; ensure we have visible points.
    const ticks = page.locator('.eyeloss-timeline__tick');
    expect(await ticks.count()).toBeGreaterThan(0);

    const targetTick = page.locator('.eyeloss-timeline__tick', {
      has: page.locator('.eyeloss-timeline__tick-id', { hasText: 'c0ffee11' }),
    });
    await expect(targetTick).toHaveCount(1);

    // Hover over a tick to update the fixed timeline description rail
    await targetTick.hover();
    const descRail = page.locator('.eyeloss-timeline__description-text');
    await expect(descRail).toBeVisible();
    await expect(descRail).toContainText('Refactor parser pipeline');

    const tickId = targetTick.locator('.eyeloss-timeline__tick-id');
    await expect(tickId).toHaveText('c0ffee11');

    // Click a tick to select it
    await targetTick.click();
    
    // Verify input changed
    const sinceInput = page.locator('#since-input');
    await expect(sinceInput).toHaveValue('c0ffee11');
  });

  test('should support multi-select with Shift key', async ({ page }) => {
    await page.goto('/');

    const firstTick = page.locator('.eyeloss-timeline__tick', {
      has: page.locator('.eyeloss-timeline__tick-id', { hasText: 'deadbeef' }),
    });
    const secondTick = page.locator('.eyeloss-timeline__tick', {
      has: page.locator('.eyeloss-timeline__tick-id', { hasText: 'c0ffee11' }),
    });
    await expect(firstTick).toHaveCount(1);
    await expect(secondTick).toHaveCount(1);

    // Select one commit, then shift-add another
    await firstTick.click();
    await page.keyboard.down('Shift');
    await secondTick.click();
    await page.keyboard.up('Shift');

    // Verify aggregate revset in input
    const sinceInput = page.locator('#since-input');
    await expect(sinceInput).toHaveValue('deadbeef | c0ffee11');
  });
});
