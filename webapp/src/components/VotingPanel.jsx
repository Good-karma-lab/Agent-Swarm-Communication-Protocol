export default function VotingPanel({ voting }) {
  const scrub = (s) => String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, '[agent]')
  return (
    <div className="card">
      <h2>Voting Process Logs</h2>
      <table>
        <thead>
          <tr>
            <th>Task</th>
            <th>Phase</th>
            <th>Commits</th>
            <th>Reveals</th>
            <th>Plans</th>
          </tr>
        </thead>
        <tbody>
          {(voting?.rfp || []).map((item) => (
            <tr key={item.task_id}>
              <td className="mono">{item.task_id}</td>
              <td>{item.phase}</td>
              <td>{item.commit_count}/{item.expected_proposers || 0}</td>
              <td>{item.reveal_count}/{item.expected_proposers || 0}</td>
              <td>
                {(item.plans || [])
                  .map((p) => `${p.plan_id} by ${scrub(p.proposer_name || 'unknown')}`)
                  .join(', ')}
                {(item.missing_proposer_names || []).length ? ` | missing proposers: ${(item.missing_proposer_names || []).join(', ')}` : ''}
                {(item.missing_voter_names || []).length ? ` | missing voters: ${(item.missing_voter_names || []).join(', ')}` : ''}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
