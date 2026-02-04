import { test, expect } from '@playwright/test';

test('plus sign should only appear if there are hidden outgoing children', async ({ page }) => {
    await page.goto('/');

    // 1. Enable Exploration Mode
    await page.getByLabel('Exploration Mode').check();

    // 2. Expand chain to reach CodingStyle
    // SoftwareApplication -> Developer -> CodingStyle

    const rootNode = page.locator('.react-flow__node').filter({ hasText: 'SoftwareApplication' });
    await rootNode.dispatchEvent('click');

    const devNode = page.locator('.react-flow__node').filter({ hasText: 'Developer' });
    await devNode.dispatchEvent('click');

    const codingStyleNode = page.locator('.react-flow__node').filter({ hasText: 'CodingStyle' });
    await expect(codingStyleNode).toBeVisible();

    // 3. CodingStyle has a hidden parent "GoogleCodingStyle"
    // The bug fix ensures it DOES NOT show a plus sign because it has no hidden OUTGOING children.
    const plusIndicator = codingStyleNode.locator('span', { hasText: '➕' });

    // This should now NOT be visible
    await expect(plusIndicator).not.toBeVisible();
});
