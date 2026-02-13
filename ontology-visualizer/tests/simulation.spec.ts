
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

        // 5. Verify Animation Classes on Graph
        // Check if any node has the 'node-produced' class
        const producedNode = page.locator('.react-flow__node.node-produced').first();
        await expect(producedNode).toBeVisible();
        console.log('TEST: node-produced class verified.');

        // 6. Test Playback: Click Forward
        console.log('TEST: Testing playback controls...');
        const forwardBtn = page.locator('button').filter({ has: page.locator('svg.lucide-arrow-right') }).last();
        await forwardBtn.click();

        // Check for node-highlighted class
        const highlightedNode = page.locator('.react-flow__node.node-highlighted');
        await expect(highlightedNode).toBeVisible();
        console.log('TEST: node-highlighted class verified.');

        console.log('TEST: Simulation playback and animations verified. Success.');
    });
});
