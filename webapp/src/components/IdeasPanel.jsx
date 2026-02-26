export default function IdeasPanel({ recommendations }) {
  return (
    <div className="card">
      <h2>Proposed Next UI Features</h2>
      <ul>
        {(recommendations?.recommended_features || []).map((item, idx) => (
          <li key={`${item}-${idx}`}>{item}</li>
        ))}
      </ul>
    </div>
  )
}
