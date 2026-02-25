// @ts-check
const { defineConfig } = require('@playwright/test')

module.exports = defineConfig({
  testDir: '.',
  timeout: 120000,
  retries: 0,
  use: {
    headless: true,
    baseURL: 'http://127.0.0.1:22371'
  }
})
