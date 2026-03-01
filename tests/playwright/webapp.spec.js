import { test, expect } from '@playwright/test'

const BASE = 'http://localhost:5173'

test.describe('WWS Webapp', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(BASE)
    await page.waitForLoadState('networkidle')
  })

  test('shows WWS brand in header', async ({ page }) => {
    await expect(page.locator('.brand')).toContainText('WWS')
  })

  test('shows three center view tabs', async ({ page }) => {
    const tabs = page.locator('.view-tab')
    await expect(tabs).toHaveCount(3)
    await expect(tabs.nth(0)).toContainText('Graph')
    await expect(tabs.nth(1)).toContainText('Directory')
    await expect(tabs.nth(2)).toContainText('Activity')
  })

  test('switches to Directory view', async ({ page }) => {
    await page.click('.view-tab:nth-child(2)')
    await expect(page.locator('.directory-container')).toBeVisible()
    await expect(page.locator('.search-input')).toBeVisible()
  })

  test('switches to Activity view', async ({ page }) => {
    await page.click('.view-tab:nth-child(3)')
    await expect(page.locator('.activity-container')).toBeVisible()
    const actTabs = page.locator('.activity-tab')
    await expect(actTabs).toHaveCount(4)
  })

  test('shows left column with My Agent section', async ({ page }) => {
    await expect(page.locator('.col-left')).toBeVisible()
    await expect(page.locator('.identity-name')).toBeVisible()
  })

  test('shows right column live stream', async ({ page }) => {
    await expect(page.locator('.col-right')).toBeVisible()
    await expect(page.locator('.stream-header')).toContainText('Live Stream')
  })

  test('opens Name Registry panel from left column', async ({ page }) => {
    await page.click('.add-name-btn')
    await expect(page.locator('.slide-panel.open')).toBeVisible()
    await expect(page.locator('.panel-title')).toContainText('Name Registry')
    await page.click('.panel-close')
    await expect(page.locator('.slide-panel.open')).toHaveCount(0)
  })

  test('opens Audit panel from header', async ({ page }) => {
    await page.click('.btn:has-text("Audit")')
    await expect(page.locator('.slide-panel.open')).toBeVisible()
    await page.click('.panel-close')
  })

  test('no console errors on load', async ({ page }) => {
    const errors = []
    page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()) })
    await page.goto(BASE)
    await page.waitForTimeout(2000)
    expect(errors).toHaveLength(0)
  })
})
