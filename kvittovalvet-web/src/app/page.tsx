'use client';
import { useEffect, useState } from 'react';
import AppShell from '@/components/AppShell';
import StatCard from '@/components/StatCard';
import { CheckCircle, AlertTriangle, Clock, TrendingUp, RefreshCw } from 'lucide-react';
import { api } from '@/lib/api';

interface RoiData {
  auto_booked: number;
  manual_required: number;
  automation_rate: number;
  hours_saved: number;
  cost_saved_sek: number;
}

export default function Dashboard() {
  const [roi, setRoi]         = useState<RoiData | null>(null);
  const [health, setHealth]   = useState<{ status: string; version: string } | null>(null);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    setLoading(true);
    try {
      const [r, h] = await Promise.all([api.roi(), api.health()]);
      setRoi(r);
      setHealth(h);
    } catch { /* offline */ }
    setLoading(false);
  };

  useEffect(() => { load(); }, []);

  const autoRate = roi ? Math.round(roi.automation_rate * 100) : 0;

  return (
    <AppShell>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-white">Dashboard</h1>
          <p className="text-xs text-white/40 mt-0.5">Economic evidence overview</p>
        </div>
        <div className="flex items-center gap-3">
          {health && (
            <span className="flex items-center gap-1.5 text-xs text-green-400">
              <span className="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse" />
              API v{health.version}
            </span>
          )}
          <button
            onClick={load}
            className="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-white/50 hover:text-white transition-all"
          >
            <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
          </button>
        </div>
      </div>

      {/* Stats grid */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
        <StatCard
          label="Automatiserat"
          value={loading ? '–' : `${autoRate}%`}
          sub="av alla transaktioner"
          accent="green"
          icon={<CheckCircle size={16} />}
        />
        <StatCard
          label="Auto-bokade"
          value={loading ? '–' : (roi?.auto_booked ?? 0)}
          sub="verifierade"
          accent="indigo"
          icon={<TrendingUp size={16} />}
        />
        <StatCard
          label="Kräver åtgärd"
          value={loading ? '–' : (roi?.manual_required ?? 0)}
          sub="manuell granskning"
          accent="yellow"
          icon={<AlertTriangle size={16} />}
        />
        <StatCard
          label="Tid sparad"
          value={loading ? '–' : `${roi?.hours_saved ?? 0}h`}
          sub={`≈ ${roi?.cost_saved_sek?.toLocaleString('sv-SE') ?? 0} kr`}
          accent="green"
          icon={<Clock size={16} />}
        />
      </div>

      {/* Evidence pipeline */}
      <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5 mb-4">
        <h2 className="text-sm font-medium text-white/70 mb-4 uppercase tracking-wider">Evidence Pipeline</h2>
        <div className="flex items-center gap-2 flex-wrap">
          {[
            { label: 'Banktransaktioner', state: 'VERIFIED',   color: 'bg-green-500' },
            { label: 'ERP-verifikationer', state: 'SYNCING',   color: 'bg-indigo-500' },
            { label: 'Saknade underlag',   state: 'RETRIEVAL', color: 'bg-yellow-500' },
            { label: 'Eskaleringskö',      state: 'QUEUED',    color: 'bg-orange-500' },
          ].map(({ label, state, color }) => (
            <div key={label} className="flex items-center gap-2 px-3 py-2 rounded-lg bg-white/5 text-xs">
              <span className={`w-2 h-2 rounded-full ${color} animate-pulse`} />
              <span className="text-white/70">{label}</span>
              <span className="text-white/30">·</span>
              <span className="text-white/40">{state}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Eskalering-tratten */}
      <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
        <h2 className="text-sm font-medium text-white/70 mb-4 uppercase tracking-wider">Eskaleringstunnel</h2>
        <div className="space-y-2">
          {[
            { step: '1', label: 'API Retrieval',   pct: 72, done: true },
            { step: '2', label: 'Peppol Request',  pct: 48, done: true },
            { step: '3', label: 'AI-mail',         pct: 31, done: false },
            { step: '4', label: 'SMS / Röstsamtal',pct: 12, done: false },
            { step: '5', label: 'Rek. brev',       pct: 4,  done: false },
            { step: '6', label: 'Juridisk export', pct: 1,  done: false },
          ].map(({ step, label, pct, done }) => (
            <div key={step} className="flex items-center gap-3">
              <span className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold flex-shrink-0 ${
                done ? 'bg-green-500/20 text-green-400' : 'bg-white/5 text-white/30'
              }`}>{step}</span>
              <span className="text-xs text-white/50 w-32 flex-shrink-0">{label}</span>
              <div className="flex-1 h-1.5 bg-white/5 rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full ${done ? 'bg-green-500' : 'bg-indigo-500/50'}`}
                  style={{ width: `${pct}%` }}
                />
              </div>
              <span className="text-[10px] text-white/30 w-6 text-right">{pct}%</span>
            </div>
          ))}
        </div>
      </div>
    </AppShell>
  );
}
