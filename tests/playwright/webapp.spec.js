const { test, expect } = require('@playwright/test')
const { spawn } = require('node:child_process')

let connector

async function waitForHealth(page, tries = 40) {
  for (let i = 0; i < tries; i += 1) {
    try {
      const res = await page.request.get('/api/health')
      if (res.ok()) {
        return
      }
    } catch (_) {
      // ignore
    }
    await new Promise((r) => setTimeout(r, 500))
  }
  throw new Error('Web dashboard did not become healthy in time')
}

test.beforeAll(async () => {
  connector = spawn(
    '../../target/release/openswarm-connector',
    [
      '--listen',
      '/ip4/127.0.0.1/tcp/22100',
      '--rpc',
      '127.0.0.1:22370',
      '--files-addr',
      '127.0.0.1:22371',
      '--agent-name',
      'playwright-web'
    ],
    { cwd: process.cwd(), stdio: 'ignore' }
  )
})

test.afterAll(async () => {
  if (connector && !connector.killed) {
    connector.kill('SIGTERM')
  }
})

test('renders dashboard and submits task', async ({ page }) => {
  await waitForHealth(page)

  await page.goto('/')
  await expect(page.getByText('OpenSwarm Web Console')).toBeVisible()

  await page.locator('textarea').first().fill('Playwright UI task submission check')
  await page.getByRole('button', { name: 'Submit' }).click()

  await page.getByRole('button', { name: 'task' }).click()
  await expect(page.getByText('Task Timeline')).toBeVisible()

  await page.getByRole('button', { name: 'topology' }).click()
  await expect(page.locator('#topologyGraph')).toBeVisible()

  const health = await page.request.get('/api/health')
  expect(health.ok()).toBeTruthy()
})
