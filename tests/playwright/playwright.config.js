// @ts-check
const { defineConfig } = require('@playwright/test')

module.exports = defineConfig({
  testDir: '.',
  timeout: 120000,
  retries: 0,
  use: {
    headless: process.env.PLAYWRIGHT_HEADED === '1' ? false : true,
    baseURL: process.env.WEB_BASE_URL || 'http://127.0.0.1:22371'
  }
})
