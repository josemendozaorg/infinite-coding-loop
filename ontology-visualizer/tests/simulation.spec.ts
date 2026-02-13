
import { test, expect } from '@playwright/test';

test.describe('Execution Simulation', () => {
    test.beforeEach(async ({ page }) => {
        // These actions are now performed within the test for better logging and timeout control.
        // await page.goto('http://localhost:4173/');
        // // Wait for graph to load
        // await page.waitForSelector('.react-flow__node');
    });


    test('should open simulation panel and run simulation', async ({ page }) => {
        test.setTimeout(15000); // Fail faster
        console.log('TEST: Starting simulation test...');

        // 1. Click Simulate button
        console.log('TEST: Navigating to page...');
        await page.goto('http://localhost:4173/');
        console.log('TEST: Page loaded. Waiting for graph...');
        await page.waitForSelector('.react-flow__node');
        console.log('TEST: Graph loaded.');

        const simulateBtn = page.getByRole('button', { name: 'Simulate' });
        await expect(simulateBtn).toBeVisible();
        console.log('TEST: Clicking Simulate button...');
        await simulateBtn.click();

        // 2. Verify Panel Opens
        console.log('TEST: Waiting for panel...');
        const panel = page.locator('.side-panel.left-panel');
        await expect(panel).toBeVisible();
        await expect(panel).toContainText('Execution Simulation');
        console.log('TEST: Panel opened.');

        // 3. Click Run Simulation
        const runBtn = page.getByRole('button', { name: 'Run Simulation' });
        await expect(runBtn).toBeVisible();

        // Capture console logs to debug 0 steps issue
        const consoleLogs: string[] = [];
        page.on('console', msg => {
            const text = msg.text();
            consoleLogs.push(text);
            // Also print [Simulation] logs immediately to stdout
            if (text.includes('[Simulation]')) {
                console.log(`BROWSER: ${text}`);
            }
        });

        console.log('TEST: Clicking Run Simulation...');
        await runBtn.click();
        console.log('TEST: Clicked Run Simulation. Waiting for results...');

        // 4. Verify Steps Appear
        const initialMsg = page.getByText('Click "Run Simulation" to see the predicted execution path.');
        await expect(initialMsg).not.toBeVisible({ timeout: 5000 });

        // Verify at least one step exists
        const step = page.locator('.panel-content > div > div').first();
        await expect(step).toBeVisible();
        console.log('TEST: Steps produced.');

        // 5. Verify Progressive Visibility (Artifact should be hidden initially)
        const archStyleNode = page.locator('.react-flow__node').filter({ hasText: 'ArchitectureStyle' });
        await expect(archStyleNode).not.toBeVisible();
        console.log('TEST: Artifact hidden before creation (Progressive Visibility).');

        // 6. Test Playback: Click Forward to Step 1 (Architect creates ArchitectureStyle)
        console.log('TEST: Testing playback controls...');
        const forwardBtn = page.locator('button').filter({ has: page.locator('svg.lucide-arrow-right') }).last();
        await forwardBtn.click();

        // Now ArchitectureStyle should be visible and highlighted
        await expect(archStyleNode).toBeVisible();
        await expect(archStyleNode).toHaveClass(/node-highlighted/);
        console.log('TEST: Artifact appeared and highlighted after creation step.');

        console.log('TEST: Simulation playback and animations verified. Success.');
    });
});
