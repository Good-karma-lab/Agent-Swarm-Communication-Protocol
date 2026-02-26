const { test, expect } = require('@playwright/test')

test('30-agent swarm web console shows requested capabilities', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByText('OpenSwarm Web Console')).toBeVisible()

  // 1) Expandable hierarchy
  await page.getByRole('button', { name: 'hierarchy' }).click()
  await expect(page.getByText('Expandable Hierarchy')).toBeVisible()

  // 4) Submit task from UI
  const taskText = `Playwright real e2e task ${Date.now()}`
  await page.locator('textarea').first().fill(taskText)
  await page.getByRole('button', { name: 'Submit' }).click()

  // 2) Voting logs
  await page.getByRole('button', { name: 'voting' }).click()
  await expect(page.getByText('Voting Process Logs')).toBeVisible()

  // 3) P2P message logs
  await page.getByRole('button', { name: 'messages' }).click()
  await expect(page.getByText('Peer-to-Peer Debug Logs')).toBeVisible()

  // 5) Task forensics panel
  await page.getByRole('button', { name: 'task' }).click()
  await expect(page.getByText('Task Timeline Replay')).toBeVisible()
  await expect(page.getByText('Task DAG')).toBeVisible()
  await expect(page.getByText('Root Task + Aggregation State')).toBeVisible()

  // 6) Topology visualization
  await page.getByRole('button', { name: 'topology' }).click()
  await expect(page.locator('#topologyGraph')).toBeVisible()

  // 7) UI feature recommendations
  await page.getByRole('button', { name: 'ideas' }).click()
  await expect(page.getByText('Proposed Next UI Features')).toBeVisible()

  // Audit visibility
  await page.getByRole('button', { name: 'audit' }).click()
  await expect(page.getByText('Operator Audit Log')).toBeVisible()
})
