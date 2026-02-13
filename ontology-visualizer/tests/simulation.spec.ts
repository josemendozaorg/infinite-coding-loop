
import { test, expect } from '@playwright/test';

test.describe('Execution Simulation', () => {
    test.beforeEach(async ({ page }) => {
        // These actions are now performed within the test for better logging and timeout control.
        // await page.goto('http://localhost:4173/');
        // // Wait for graph to load
        // await page.waitForSelector('.react-flow__node');
    });


    test('should open simulation panel and run simulation', async ({ page }) => {
        // Use baseURL with testing=true to disable animations
        await page.goto('/?testing=true');

        // Wait for graph to load
        await page.waitForSelector('.react-flow__node', { timeout: 10000 });

        const simulateBtn = page.getByRole('button', { name: 'Simulate' });
        await simulateBtn.click();

        // Verify Panel Opens
        const panel = page.locator('.side-panel.left-panel');
        await expect(panel).toBeVisible();
        await expect(panel).toContainText('Execution Simulation');

        // Click Run Simulation
        const runBtn = page.getByRole('button', { name: 'Run Simulation' });
        await runBtn.click();

        // 4. Verify Steps Appear (Artifact hidden before creation)
        const archStyleNode = page.locator('.react-flow__node').filter({ hasText: 'ArchitectureStyle' });
        await expect(archStyleNode).not.toBeVisible();

        // 5. Test Playback: Click Forward to Step 1
        const forwardBtn = page.locator('button').filter({ has: page.locator('svg.lucide-arrow-right') }).last();
        await forwardBtn.click();

        // Now ArchitectureStyle should be visible and highlighted
        await expect(archStyleNode).toBeVisible();
        await expect(archStyleNode).toHaveClass(/node-highlighted/);

        // 6. Verify Clean View (Decluttering)
        const usesEdge = page.locator('.react-flow__edge-path');
        const initialEdgeCount = await usesEdge.count();

        // Toggle 'Show All Relationships'
        await page.getByLabel('Show All Relationships').check();

        // Edge count should increase quickly since animations are disabled
        await expect(async () => {
            expect(await usesEdge.count()).toBeGreaterThan(initialEdgeCount);
        }).toPass({ timeout: 2000 });

        // 7. Verify Path Flow Layout
        await page.getByRole('button', { name: 'Path Flow' }).click();
        await expect(page.getByRole('button', { name: 'Ontology' })).toBeVisible();
    });
});
