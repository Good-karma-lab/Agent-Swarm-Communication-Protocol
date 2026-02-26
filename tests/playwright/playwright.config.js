// @ts-check
const { defineConfig } = require('@playwright/test')

module.exports = defineConfig({
  testDir: '.',
  timeout: 120000,
  retries: 1,
  reporter: [['list'], ['html', { outputFolder: 'playwright-report', open: 'never' }]],
  use: {
    headless: process.env.PLAYWRIGHT_HEADED === '1' ? false : true,
    baseURL: process.env.WEB_BASE_URL || 'http://127.0.0.1:22371',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    launchOptions: {
      args: [
        '--disable-dev-shm-usage',
        '--disable-gpu',
        '--no-sandbox',
        '--disable-setuid-sandbox'
      ]
    }
  }
})
