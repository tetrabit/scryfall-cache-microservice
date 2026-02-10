import { useEffect, useState } from 'react'
import './App.css'

type ApiError = {
  code: string
  message: string
  request_id: string
  details?: unknown
}

type ApiResponse<T> = {
  success: boolean
  data: T | null
  error: ApiError | null
}

type AdminOverview = {
  service: string
  version: string
  instance_id: string
  cards_total: number
  cache_entries_total: number
  bulk_last_import: string | null
  bulk_reload_recommended: boolean
}

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, init)
  const text = await res.text()
  const json = text ? (JSON.parse(text) as T) : ({} as T)
  if (!res.ok) {
    throw new Error(`${res.status} ${res.statusText}: ${text}`)
  }
  return json
}

function App() {
  const [overview, setOverview] = useState<AdminOverview | null>(null)
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [reloading, setReloading] = useState(false)

  async function refresh() {
    try {
      setError(null)
      const resp = await fetchJson<ApiResponse<AdminOverview>>(
        '/api/admin/stats/overview'
      )
      if (!resp.success) {
        setError(resp.error?.message ?? 'Unknown error')
        setOverview(null)
        return
      }
      setOverview(resp.data)
      setLastUpdated(new Date())
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  async function triggerReload() {
    if (reloading) return
    const ok = window.confirm(
      'Trigger bulk data reload now? This can take a few minutes and may increase DB load.'
    )
    if (!ok) return
    try {
      setReloading(true)
      setError(null)
      await fetchJson('/admin/reload', { method: 'POST' })
      await refresh()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setReloading(false)
    }
  }

  useEffect(() => {
    void refresh()
    const id = window.setInterval(() => void refresh(), 5000)
    return () => window.clearInterval(id)
  }, [])

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">
          <div className="brand__mark" aria-hidden />
          <div>
            <div className="brand__title">Scryfall Cache</div>
            <div className="brand__subtitle">Admin</div>
          </div>
        </div>
        <div className="topbar__meta">
          <a className="link" href="/api-docs" target="_blank" rel="noreferrer">
            OpenAPI
          </a>
          <a className="link" href="/metrics" target="_blank" rel="noreferrer">
            Metrics
          </a>
          <a className="link" href="/health/ready" target="_blank" rel="noreferrer">
            Readiness
          </a>
        </div>
      </header>

      <main className="grid">
        <section className="card">
          <div className="card__head">
            <h2>Overview</h2>
            <div className="pill">
              {lastUpdated ? `Updated ${lastUpdated.toLocaleTimeString()}` : 'Loading...'}
            </div>
          </div>

          {error ? (
            <div className="alert">
              <div className="alert__title">Error</div>
              <div className="alert__body">{error}</div>
            </div>
          ) : null}

          <div className="kv">
            <div className="kv__row">
              <div className="kv__k">Service</div>
              <div className="kv__v">{overview?.service ?? '-'}</div>
            </div>
            <div className="kv__row">
              <div className="kv__k">Version</div>
              <div className="kv__v">{overview?.version ?? '-'}</div>
            </div>
            <div className="kv__row">
              <div className="kv__k">Instance</div>
              <div className="kv__v">{overview?.instance_id ?? '-'}</div>
            </div>
          </div>
        </section>

        <section className="card">
          <div className="card__head">
            <h2>Data</h2>
            <div className="actions">
              <button className="btn" onClick={() => void refresh()}>
                Refresh
              </button>
              <button
                className="btn btn--danger"
                onClick={() => void triggerReload()}
                disabled={reloading}
              >
                {reloading ? 'Reloading...' : 'Bulk Reload'}
              </button>
            </div>
          </div>

          <div className="metrics">
            <div className="metric">
              <div className="metric__label">Cards</div>
              <div className="metric__value">
                {overview ? overview.cards_total.toLocaleString() : '–'}
              </div>
            </div>
            <div className="metric">
              <div className="metric__label">Query Cache Entries</div>
              <div className="metric__value">
                {overview ? overview.cache_entries_total.toLocaleString() : '–'}
              </div>
            </div>
          </div>

          <div className="kv">
            <div className="kv__row">
              <div className="kv__k">Last Bulk Import</div>
              <div className="kv__v">{overview?.bulk_last_import ?? 'unknown'}</div>
            </div>
            <div className="kv__row">
              <div className="kv__k">Reload Recommended</div>
              <div className="kv__v">
                {overview ? (overview.bulk_reload_recommended ? 'yes' : 'no') : '-'}
              </div>
            </div>
          </div>
        </section>

        <section className="card card--wide">
          <div className="card__head">
            <h2>Notes</h2>
          </div>
          <ul className="notes">
            <li>
              This UI is intentionally lightweight: it reads backend JSON endpoints and links out to
              Prometheus metrics and OpenAPI docs.
            </li>
            <li>
              Authentication is not wired yet. Treat admin endpoints as trusted-network only until
              API key auth exists.
            </li>
          </ul>
        </section>
      </main>

      <footer className="footer">
        <div className="footer__left">Scale-ready, not scaled.</div>
        <div className="footer__right">
          <span className="muted">Backend:</span> <code>/api/admin/stats/overview</code>
        </div>
      </footer>
    </div>
  )
}

export default App
