import { test, expect } from '@playwright/test';

test.describe('Edge Grouping', () => {
    test('should group multiple relationships between same nodes into a single edge', async ({ page }) => {
        // Navigate to the visualizer
        await page.goto('http://localhost:5173');

        // Wait for the graph to load
        await page.waitForSelector('.react-flow__renderer');

        // The Developer -> Code relationships should be grouped
        // Labels should be sorted alphabetically: "creates, improves, verifies"
        const groupedLabel = page.getByText(/creates, improves, verifies/);

        // We expect to find exactly one label containing all three relationships
        await expect(groupedLabel).toBeVisible({ timeout: 15000 });

        // Verify there is an edge for Developer and Code
        // React Flow edges often have classes like .react-flow__edge
        const edge = page.locator('.react-flow__edge');
        await expect(edge.first()).toBeVisible();
    });
});
