export default function VotingPanel({ voting }) {
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
              <td>{item.commit_count}</td>
              <td>{item.reveal_count}</td>
              <td>{(item.plans || []).map((p) => p.plan_id).join(', ')}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
