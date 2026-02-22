import { test, expect } from '@playwright/test';

test('exploration mode folding should hide all descendants', async ({ page }) => {
    await page.goto('/');

    // 1. Enable Exploration Mode
    await page.getByLabel('Exploration Mode').check();

    // 2. Expand SoftwareApplication (Root)
    const rootNode = page.locator('.react-flow__node').filter({ hasText: 'SoftwareApplication' });
    await rootNode.dispatchEvent('click');

    // Check that Developer appeared
    const devNode = page.locator('.react-flow__node').filter({ hasText: 'Developer' });
    await expect(devNode).toBeVisible();

    // 3. Expand Developer to see Code
    await devNode.dispatchEvent('click');
    const codeNode = page.locator('.react-flow__node').filter({ hasText: 'Code' });
    await expect(codeNode).toBeVisible();

    // 4. Expand Code to see CyclomaticLowComplexity
    await codeNode.dispatchEvent('click');
    const metricNode = page.locator('.react-flow__node').filter({ hasText: 'CyclomaticLowComplexity' });
    await expect(metricNode).toBeVisible();

    // 5. Collapse SoftwareApplication (Root)
    // All descendants should disappear
    await rootNode.dispatchEvent('click');

    // Developer should be gone
    await expect(devNode).not.toBeVisible();

    // Code should be gone
    await expect(codeNode).not.toBeVisible();

    // CyclomaticLowComplexity should be gone
    await expect(metricNode).not.toBeVisible();
});
