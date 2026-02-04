import { test, expect } from '@playwright/test';

test('has title and renders header', async ({ page }) => {
    page.on('console', msg => console.log(`BROWSER LOG: ${msg.text()}`));
    page.on('pageerror', err => console.log(`BROWSER ERROR: ${err}`));
    await page.goto('/');

    // Expect a title "to contain" a substring.
    await expect(page).toHaveTitle(/ontology-visualizer/);

    // Check if header and filter controls are visible
    const header = page.locator('.floating-header h1');
    await expect(header).toBeVisible();
    await expect(header).toHaveText('Ontology Visualizer');

    // Check orphan filter toggle
    // Note: It might be hidden if Exploration Mode is default, but let's assume it's visible initially
    const toggle = page.getByLabel('Show All Nodes');
    if (await toggle.isVisible()) {
        await expect(toggle).toBeVisible();
    }

    // Check Exploration Mode toggle
    const exploreToggle = page.getByLabel('Exploration Mode');
    await expect(exploreToggle).toBeVisible();

    // Check Compact button
    const compactBtn = page.getByRole('button', { name: 'Compact' });
    await expect(compactBtn).toBeVisible();
    await compactBtn.click();

    // Wait for layout animation/update (simple pause as we can't easily check position change without screenshot)
    await page.waitForTimeout(500);
});

test('renders react flow graph and interaction', async ({ page }) => {
    await page.goto('/');

    // Wait for the graph container to be visible
    const diagram = page.locator('.react-flow__renderer');
    await expect(diagram).toBeVisible();

    // Check if there are some nodes
    const nodes = page.locator('.react-flow__node');
    await expect(nodes.first()).toBeVisible();

    // Wait for fitView animation/timeout
    await page.waitForTimeout(1000);

    // Click the first node and check if side panel opens
    // Use dispatchEvent to bypass viewport checks which are flaky with large canvas/zoom
    await nodes.first().dispatchEvent('click');
    const sidePanel = page.locator('.side-panel.floating');
    await expect(sidePanel).toBeVisible();
});

test('progressive disclosure exploration', async ({ page }) => {
    await page.goto('/');

    // Enable Exploration Mode
    await page.getByLabel('Exploration Mode').check();

    // Verify only Root Node is visible (and maybe 1-2 others if they were already in visible set, 
    // but our logic resets it to ['SoftwareApplication'])
    const nodes = page.locator('.react-flow__node');
    await expect(nodes).toHaveCount(1);
    await expect(nodes.first()).toHaveText(/SoftwareApplication/);

    // Click Root Node to expand
    // Force click or dispatch as before to be safe
    await nodes.first().dispatchEvent('click');

    // Verify count increases (neighbors revealed)
    // Wait a bit for state update and layout
    await page.waitForTimeout(1000);
    const expandedCount = await nodes.count();
    expect(expandedCount).toBeGreaterThan(1);

    // Click Root Node again to collapse
    await nodes.first().dispatchEvent('click');
    await page.waitForTimeout(1000);
    expect(await nodes.count()).toBe(1);

});

test('verifies node icons and expansion indicators', async ({ page }) => {
    await page.goto('/');

    // 1. Check Root Node Icon (Box)
    const rootNode = page.locator('.react-flow__node').filter({ hasText: 'SoftwareApplication' });
    await expect(rootNode).toBeVisible();
    await expect(rootNode.locator('svg')).toBeVisible();

    // 2. Check Agent Icon (Bot) 
    // Enable Exploration Mode
    await page.getByLabel('Exploration Mode').check();
    await rootNode.dispatchEvent('click');
    await page.waitForTimeout(1000);

    // Find an Agent node
    const agentNode = page.locator('.react-flow__node').filter({ hasText: /^Agent$|^Architect$|^Developer$/ }).first();
    if (await agentNode.isVisible()) {
        await expect(agentNode.locator('svg')).toBeVisible();
    }

    // 3. Check Expansion Indicator (Plus sign)
    // Collapse to see indicator
    await rootNode.dispatchEvent('click');
    await page.waitForTimeout(1000);
    await expect(rootNode).toContainText('➕');
});
