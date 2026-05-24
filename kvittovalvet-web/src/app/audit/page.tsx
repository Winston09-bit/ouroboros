'use client';
import AppShell from '@/components/AppShell';
import { ShieldCheck, Download, Eye } from 'lucide-react';

const log = [
  { ts: '2026-05-24 11:31', actor: 'system',          action: 'auto_booked',    object: 'TXN-1001', confidence: 0.97, source: 'fortnox'  },
  { ts: '2026-05-24 11:30', actor: 'agent:receipt',   action: 'matched',        object: 'TXN-1002', confidence: 0.91, source: 'nordea'   },
  { ts: '2026-05-23 14:37', actor: 'agent:receipt',   action: 'escalated',      object: 'TXN-1003', confidence: 0.00, source: 'revolut'  },
  { ts: '2026-05-23 09:05', actor: 'user:winston',    action: 'reviewed',       object: 'INV-0056', confidence: 1.00, source: 'fortnox'  },
  { ts: '2026-05-22 16:00', actor: 'system',          action: 'vat_verified',   object: 'TXN-0997', confidence: 0.99, source: 'fortnox'  },
  { ts: '2026-05-22 15:55', actor: 'agent:vat',       action: 'mismatch_found', object: 'TXN-0994', confidence: 0.72, source: 'skatteverket' },
];

const actionColor: Record<string, string> = {
  auto_booked:    'text-green-400',
  matched:        'text-green-400',
  escalated:      'text-orange-400',
  reviewed:       'text-blue-400',
  vat_verified:   'text-green-400',
  mismatch_found: 'text-red-400',
};

export default function Audit() {
  return (
    <AppShell>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-white">Revision</h1>
          <p className="text-xs text-white/40 mt-0.5">Immutable audit trail · append-only</p>
        </div>
        <button className="flex items-center gap-2 px-3 py-2 text-xs bg-indigo-600/20 border border-indigo-500/30 text-indigo-300 rounded-lg hover:bg-indigo-600/30 transition-all">
          <Download size={13} />
          Exportera juridiskt paket
        </button>
      </div>

      {/* Integrity banner */}
      <div className="flex items-center gap-3 px-4 py-3 rounded-xl border border-green-500/20 bg-green-500/5 mb-6">
        <ShieldCheck size={16} className="text-green-400 flex-shrink-0" />
        <div>
          <p className="text-xs font-medium text-green-300">Revisionskedjan är intakt</p>
          <p className="text-[10px] text-green-400/50">{log.length} händelser · kryptografiskt signerade · SHA-256</p>
        </div>
      </div>

      {/* Log */}
      <div className="rounded-xl border border-white/5 overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-white/5 bg-white/[0.02]">
              {['Tid', 'Aktör', 'Händelse', 'Objekt', 'Säkerhet', 'Källa', ''].map(h => (
                <th key={h} className="text-left px-4 py-3 text-xs font-medium text-white/40 uppercase tracking-wider">{h}</th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-white/5">
            {log.map((row, i) => (
              <tr key={i} className="hover:bg-white/[0.02] transition-colors group">
                <td className="px-4 py-3 text-xs text-white/30 font-mono">{row.ts}</td>
                <td className="px-4 py-3 text-xs text-white/50">{row.actor}</td>
                <td className="px-4 py-3">
                  <span className={`text-xs font-medium ${actionColor[row.action] ?? 'text-white/50'}`}>{row.action}</span>
                </td>
                <td className="px-4 py-3 text-xs font-mono text-white/60">{row.object}</td>
                <td className="px-4 py-3 text-xs text-right">
                  {row.confidence > 0
                    ? <span className="text-green-400">{Math.round(row.confidence * 100)}%</span>
                    : <span className="text-white/20">–</span>}
                </td>
                <td className="px-4 py-3">
                  <span className="px-2 py-0.5 rounded text-[10px] font-medium bg-indigo-500/10 text-indigo-400 uppercase">{row.source}</span>
                </td>
                <td className="px-4 py-3">
                  <button className="opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-white/10 text-white/40">
                    <Eye size={12} />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </AppShell>
  );
}
