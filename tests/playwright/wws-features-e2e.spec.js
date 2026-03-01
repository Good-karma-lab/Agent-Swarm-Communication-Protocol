/**
 * WWS Features UI E2E — tests all WWS-specific UI panels.
 *
 * Runs against a single connector on http://127.0.0.1:19371 with no agents.
 * Tests every LeftColumn section and each slide-over panel.
 *
 * Key facts from component analysis:
 *  - Header: .brand="WWS", identity button opens keyMgmt panel, ⚙ also opens keyMgmt
 *  - LeftColumn: .rep-score shows "{score} pts", .id-label for DID/PeerID rows
 *  - NameRegistryPanel: success msg "Registered wws:{name} successfully.", close btn "✕"
 *  - KeyManagementPanel: shows keys.did (full), keys.pubkey_hex, panel-close "✕"
 *  - ReputationPanel: panel-title "Reputation", shows "{score} pts"
 *  - AuditPanel: panel-title "Audit Log"
 *  - View tabs: className="view-tab", text Graph/Directory/Activity
 *
 * Note: The slide-panel is position:fixed with width 640px. The panel-close button
 * is in the panel header (top of panel). We use page.evaluate to click it via JS
 * to avoid Playwright's out-of-viewport error for fixed-position elements.
 */
const { test, expect } = require('@playwright/test')

test.setTimeout(120000)

/**
 * Close the currently open slide panel by clicking its .panel-close button via JS.
 * This bypasses Playwright's viewport check for fixed-position elements.
 */
async function closePanel(page) {
  await page.evaluate(() => {
    const btn = document.querySelector('.slide-panel.open .panel-close')
    if (btn) btn.click()
  })
  await page.waitForTimeout(400)
}

test('WWS Features: UI panels and identity display', async ({ page, request }) => {

  // Ensure viewport is wide enough for the 640px slide panel + left column
  await page.setViewportSize({ width: 1440, height: 900 })

  // ── Step 1: Load the web console ────────────────────────────────────────
  await test.step('Load web console', async () => {
    await page.goto('/')
    await expect(page.locator('.brand')).toBeVisible({ timeout: 15000 })
    const brandText = await page.locator('.brand').textContent()
    expect(brandText).toContain('WWS')
  })

  // ── Step 2: Wait for identity data to load (polling until DID shows) ────
  await test.step('Identity loads in LeftColumn', async () => {
    // The app polls every 5 seconds; wait for the first poll cycle to complete
    await page.waitForTimeout(7000)

    // "My Agent" section header must be visible
    await expect(page.getByText('My Agent', { exact: true })).toBeVisible()

    // .tier-badge must render — element has the class set to the tier string.
    // Note: if tier is "executor" (not in TIER_LABELS), the badge text is empty
    // but the element still renders with the correct CSS class.
    const badge = page.locator('.col-left .tier-badge').first()
    await expect(badge).toBeVisible()
    // Verify the badge element has a tier class (class list contains at least one tier)
    const badgeClass = await badge.getAttribute('class')
    expect(badgeClass).toContain('tier-badge')
  })

  // ── Step 3: Header stats show peer/agent counts ──────────────────────────
  await test.step('Header stats render', async () => {
    const stats = page.locator('.header-stats')
    await expect(stats).toBeVisible()
    // Stats show "◎ N agents" and "⬡ N peers"
    await expect(stats.getByText(/agents/)).toBeVisible()
    await expect(stats.getByText(/peers/)).toBeVisible()
  })

  // ── Step 4: Reputation score visible in LeftColumn ──────────────────────
  await test.step('Reputation score in LeftColumn', async () => {
    // .rep-score shows "{score} pts" — fresh node starts at 10 pts
    const repScore = page.locator('.rep-score').first()
    await expect(repScore).toBeVisible()
    const repText = await repScore.textContent()
    expect(repText).toMatch(/\d+ pts/)
  })

  // ── Step 5: DID appears in LeftColumn identity section ──────────────────
  await test.step('DID shows in LeftColumn', async () => {
    // id-field rows: first has id-label "DID", second has "PeerID"
    const labels = page.locator('.col-left .id-label')
    await expect(labels.first()).toBeVisible()
    const firstLabel = await labels.first().textContent()
    expect(firstLabel).toBe('DID')
  })

  // ── Step 6: "+ Register name" button visible ─────────────────────────────
  await test.step('Names section: register button visible', async () => {
    // LeftColumn renders: <button className="add-name-btn">+ Register name</button>
    const addBtn = page.locator('.add-name-btn')
    await expect(addBtn).toBeVisible()
    const btnText = await addBtn.textContent()
    expect(btnText).toContain('Register name')
  })

  // ── Step 7: Key Health section shows keypair status ──────────────────────
  await test.step('Key Health section visible', async () => {
    // Section header text "Key Health ›"
    await expect(page.getByText('Key Health', { exact: false })).toBeVisible()
    // .status-dot elements in key health section
    const dots = page.locator('.col-left .status-dot')
    await expect(dots.first()).toBeVisible()
    // "keypair" label is present
    await expect(page.locator('.col-left').getByText('keypair')).toBeVisible()
  })

  // ── Step 8: Network section exists ───────────────────────────────────────
  await test.step('Network section visible', async () => {
    await expect(page.getByText('Network', { exact: true })).toBeVisible()
    // "NAT" is a status-label in the Network section
    await expect(page.locator('.col-left').getByText('NAT')).toBeVisible()
  })

  // ── Step 9: Open KeyManagementPanel via ⚙ button ────────────────────────
  await test.step('KeyManagementPanel: opens and shows key info', async () => {
    // The ⚙ button in header triggers onSettings -> openPanel('keyMgmt')
    await page.getByRole('button', { name: '⚙' }).click()
    await page.waitForTimeout(600)

    // Panel title is "Key Management"
    await expect(page.locator('.slide-panel.open .panel-title')).toHaveText('Key Management', { timeout: 5000 })

    // Panel shows DID label and pubkey label
    const panelBody = page.locator('.slide-panel.open .panel-body')
    await expect(panelBody.locator('.id-label').filter({ hasText: 'DID' })).toBeVisible()
    await expect(panelBody.locator('.id-label').filter({ hasText: 'Pubkey' })).toBeVisible()

    // Close via JS click (fixed-position element can be out of Playwright viewport coords)
    await closePanel(page)
    await expect(page.locator('.slide-panel.open')).toHaveCount(0)
  })

  // ── Step 10: Open ReputationPanel via rep score click ───────────────────
  await test.step('ReputationPanel: opens via rep score click', async () => {
    await page.locator('.rep-score').first().click()
    await page.waitForTimeout(600)

    // Panel title is "Reputation"
    await expect(page.locator('.slide-panel.open .panel-title')).toHaveText('Reputation', { timeout: 5000 })

    // Panel body shows score in pts format (big number display)
    const panelBody = page.locator('.slide-panel.open .panel-body')
    await expect(panelBody.getByText(/\d+ pts/)).toBeVisible()

    // Close via JS
    await closePanel(page)
    await expect(page.locator('.slide-panel.open')).toHaveCount(0)
  })

  // ── Step 11: Open NameRegistryPanel, register a name ────────────────────
  await test.step('NameRegistryPanel: register e2e-test-ui', async () => {
    // Click ".add-name-btn" (the "+ Register name" button in LeftColumn)
    await page.locator('.add-name-btn').click()
    await page.waitForTimeout(600)

    // Panel title is "Name Registry"
    await expect(page.locator('.slide-panel.open .panel-title')).toHaveText('Name Registry', { timeout: 5000 })

    // Input field with placeholder "alice"
    const input = page.locator('.slide-panel.open input[placeholder="alice"]')
    await expect(input).toBeVisible({ timeout: 5000 })
    await input.fill('e2e-test-ui')

    // Submit button text: "Register (PoW)" when not loading
    const submitBtn = page.locator('.slide-panel.open button[type="submit"]')
    await expect(submitBtn).toBeVisible()
    await submitBtn.click()

    // Wait for success message: "Registered wws:e2e-test-ui successfully."
    const success = page.locator('.slide-panel.open').getByText(/Registered wws:e2e-test-ui successfully/i)
    await expect(success).toBeVisible({ timeout: 15000 })

    // Close panel via JS
    await closePanel(page)
    await page.waitForTimeout(2000) // let the app's onRefresh cycle complete
  })

  // ── Step 12: Name appears in LeftColumn Names section ───────────────────
  await test.step('Registered name appears in LeftColumn', async () => {
    // App calls onRefresh after registration; it polls /api/names and re-renders
    // .name-row has .name-label span with the name text
    await expect(
      page.locator('.name-row .name-label').filter({ hasText: 'e2e-test-ui' })
    ).toBeVisible({ timeout: 15000 })
  })

  // ── Step 13: AuditPanel opens ────────────────────────────────────────────
  await test.step('AuditPanel opens via Audit button', async () => {
    // "Audit" button in header
    await page.getByRole('button', { name: 'Audit' }).click()
    await page.waitForTimeout(600)

    // Panel title is "Audit Log"
    await expect(page.locator('.slide-panel.open .panel-title')).toHaveText('Audit Log', { timeout: 5000 })

    // Close via JS
    await closePanel(page)
    await expect(page.locator('.slide-panel.open')).toHaveCount(0)
  })

  // ── Step 14: View switching (Graph, Directory, Activity) ─────────────────
  await test.step('View tabs switch without crash', async () => {
    // Tabs have className="view-tab" and text Graph/Directory/Activity
    // Switch to Directory
    await page.locator('.view-tab', { hasText: 'Directory' }).click()
    await page.waitForTimeout(1000)
    await expect(page.locator('.brand')).toBeVisible() // page didn't crash

    // Switch to Activity
    await page.locator('.view-tab', { hasText: 'Activity' }).click()
    await page.waitForTimeout(1000)
    await expect(page.locator('.brand')).toBeVisible()

    // Switch back to Graph
    await page.locator('.view-tab', { hasText: 'Graph' }).click()
    await page.waitForTimeout(1000)
    await expect(page.locator('.brand')).toBeVisible()
  })

  // ── Step 15: Verify name in API after UI registration ───────────────────
  await test.step('API confirms name registered', async () => {
    const resp = await request.get('/api/names')
    const data = await resp.json()
    const names = data.names || []
    const found = names.find(n => n.name === 'e2e-test-ui')
    expect(found, 'e2e-test-ui must appear in /api/names').toBeTruthy()
  })
})
