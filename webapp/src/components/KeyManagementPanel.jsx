export default function KeyManagementPanel({ open, keys, onClose }) {
  if (!keys) return null

  function fmtDate(ts) {
    if (!ts) return 'never'
    return new Date(ts).toLocaleString()
  }

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Key Management</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div className="section-header" style={{marginBottom:12}}>Current Keypair</div>
          <div style={{marginBottom:16}}>
            <div className="id-field"><span className="id-label">DID</span><span className="mono">{keys.did}</span></div>
            <div className="id-field"><span className="id-label">Pubkey</span><span className="mono">{keys.pubkey_hex}</span></div>
            <div className="id-field"><span className="id-label">Created</span><span className="mono">{fmtDate(keys.created_at)}</span></div>
            <div className="id-field"><span className="id-label">Rotated</span><span className="mono">{fmtDate(keys.last_rotation)}</span></div>
          </div>

          <div className="section-header" style={{marginBottom:12}}>Guardians</div>
          <div style={{marginBottom:16}}>
            <div style={{fontSize:13, color:'var(--text-muted)', marginBottom:8}}>
              {keys.guardian_count || 0} of {keys.threshold || 0} configured
            </div>
            {(keys.guardian_count || 0) === 0 && (
              <div style={{color:'var(--amber)', fontSize:12}}>
                ⚠ No guardians configured. Recovery is not possible without guardians.
              </div>
            )}
          </div>

          <div className="section-header" style={{marginBottom:12}}>Key Rotation</div>
          <div style={{marginBottom:16, fontSize:12, color:'var(--text-muted)', lineHeight:1.6}}>
            Initiating rotation will generate a new keypair. Both keypairs will sign
            messages for a 48-hour grace period. Configure your new key with peers before
            the grace period ends.
          </div>
          <button className="btn w-full" style={{marginBottom:8}}>
            Initiate Planned Rotation
          </button>

          <div style={{marginTop:24}}>
            <div className="section-header" style={{marginBottom:8}}>Emergency Revocation</div>
            <div style={{fontSize:12, color:'var(--text-muted)', marginBottom:12, lineHeight:1.6}}>
              Immediately invalidates the current keypair. Requires recovery key (BIP-39 mnemonic).
              This action is irreversible.
            </div>
            <button className="btn btn-danger w-full">
              Emergency Revocation…
            </button>
          </div>
        </div>
      </div>
    </>
  )
}
