'use client';
import { useEffect, useState } from 'react';
import AppShell from '@/components/AppShell';
import StatCard from '@/components/StatCard';
import {
  CheckCircle, AlertTriangle, Clock, TrendingUp, RefreshCw,
  CheckCircle2, XCircle, Wifi, WifiOff, ShieldCheck, AlertCircle,
} from 'lucide-react';
import {
  BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell,
} from 'recharts';
import { api } from '@/lib/api';

interface RoiData {
  auto_booked: number;
  manual_required: number;
  automation_rate: number;
  hours_saved: number;
  cost_saved_sek: number;
}

const BANK_CONNECTIONS = [
  { id: 'tink',    name: 'Tink',           env: 'sandbox', status: 'connected',    lastSync: '2 min sedan',  color: 'text-green-400',  bg: 'bg-green-500/10',  border: 'border-green-500/20' },
  { id: 'enable',  name: 'Enable Banking', env: 'prod',    status: 'connected',    lastSync: '5 min sedan',  color: 'text-green-400',  bg: 'bg-green-500/10',  border: 'border-green-500/20' },
  { id: 'revolut', name: 'Revolut',        env: 'prod',    status: 'connected',    lastSync: '1 min sedan',  color: 'text-green-400',  bg: 'bg-green-500/10',  border: 'border-green-500/20' },
  { id: 'nordea',  name: 'Nordea',         env: 'prod',    status: 'degraded',     lastSync: '12 min sedan', color: 'text-yellow-400', bg: 'bg-yellow-500/10', border: 'border-yellow-500/20' },
];

const CATEGORY_DATA = [
  { name: 'BANK', value: 13, color: '#14b8a6' },
  { name: 'RESOR', value: 18, color: '#a855f7' },
  { name: 'BYGG', value: 16, color: '#3b82f6' },
  { name: 'KONTOR', value: 10, color: '#6366f1' },
  { name: 'DRIVMEDEL', value: 10, color: '#f97316' },
  { name: 'DAGLIGVAROR', value: 8, color: '#22c55e' },
  { name: 'RESTAURANG', value: 9, color: '#ef4444' },
  { name: 'STREAMING', value: 8, color: '#ec4899' },
  { name: 'TRANSPORT', value: 8, color: '#06b6d4' },
  { name: 'TELEKOM', value: 6, color: '#eab308' },
];

const COMPLIANCE_CHECKS = [
  { label: 'Hash-kedja integritet',          status: 'ok',   detail: '1,247 kvitton verifierade' },
  { label: 'Signatur-validering',             status: 'ok',   detail: 'ES256 · nyckelrotation OK' },
  { label: 'Momsberäkning (12%/25%)',         status: 'ok',   detail: 'Alla VAT-summor stämmer' },
  { label: 'Bokföringslagen §5 compliance',   status: 'ok',   detail: 'Underlag komplett' },
  { label: 'Saknade underlag',                status: 'warn', detail: '4 transaktioner utan kvitto' },
  { label: 'Eskaleringar >14 dagar',          status: 'warn', detail: '2 ärenden försenade' },
  { label: 'API-fel (senaste 24h)',            status: 'ok',   detail: '0 fel registrerade' },
];

const CustomTooltip = ({ active, payload, label }: {
  active?: boolean;
  payload?: Array<{ value: number }>;
  label?: string;
}) => {
  if (active && payload?.length) {
    return (
      <div className="bg-[#1a1a2e] border border-white/10 rounded-lg px-3 py-2 text-xs shadow-xl">
        <p className="text-white/60 mb-0.5">{label}</p>
        <p className="text-white font-bold">{payload[0].value} merchants</p>
      </div>
    );
  }
  return null;
};

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
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
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

      {/* Three-column section */}
      <div className="grid grid-cols-3 gap-4 mb-4">
        {/* Bank connections */}
        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
          <h2 className="text-sm font-medium text-white/70 mb-4 uppercase tracking-wider">Bank-anslutningar</h2>
          <div className="space-y-2">
            {BANK_CONNECTIONS.map(bank => (
              <div
                key={bank.id}
                className={`flex items-center gap-2.5 px-3 py-2 rounded-lg ${bank.bg} border ${bank.border}`}
              >
                {bank.status === 'connected'
                  ? <Wifi size={13} className={bank.color} />
                  : <WifiOff size={13} className={bank.color} />
                }
                <div className="flex-1 min-w-0">
                  <p className="text-xs font-medium text-white truncate">{bank.name}</p>
                  <p className="text-[10px] text-white/30">{bank.lastSync}</p>
                </div>
                <span className={`text-[10px] px-1.5 py-0.5 rounded font-mono ${
                  bank.env === 'sandbox'
                    ? 'bg-purple-500/20 text-purple-400'
                    : 'bg-white/5 text-white/30'
                }`}>{bank.env}</span>
              </div>
            ))}
          </div>
          <div className="mt-3 pt-3 border-t border-white/5 flex items-center justify-between">
            <span className="text-[10px] text-white/30">3 aktiva · 1 degraderad</span>
            <span className="text-[10px] text-green-400">Open Banking PSD2</span>
          </div>
        </div>

        {/* Category distribution */}
        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
          <h2 className="text-sm font-medium text-white/70 mb-4 uppercase tracking-wider">Kategori-fördelning</h2>
          <ResponsiveContainer width="100%" height={140}>
            <BarChart data={CATEGORY_DATA} margin={{ top: 0, right: 0, left: -20, bottom: 0 }}>
              <XAxis
                dataKey="name"
                tick={{ fill: 'rgba(255,255,255,0.25)', fontSize: 7 }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                tick={{ fill: 'rgba(255,255,255,0.25)', fontSize: 8 }}
                axisLine={false}
                tickLine={false}
              />
              <Tooltip content={<CustomTooltip />} cursor={{ fill: 'rgba(255,255,255,0.03)' }} />
              <Bar dataKey="value" radius={[3, 3, 0, 0]}>
                {CATEGORY_DATA.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={entry.color} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
          <p className="text-[10px] text-white/30 text-center mt-1">106 merchants · 10 kategorier</p>
        </div>

        {/* Compliance health */}
        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
          <h2 className="text-sm font-medium text-white/70 mb-4 uppercase tracking-wider flex items-center gap-2">
            <ShieldCheck size={13} className="text-green-400" />
            Compliance Health
          </h2>
          <div className="space-y-2">
            {COMPLIANCE_CHECKS.map(check => (
              <div key={check.label} className="flex items-start gap-2">
                {check.status === 'ok'
                  ? <CheckCircle2 size={13} className="text-green-400 mt-0.5 flex-shrink-0" />
                  : <AlertCircle size={13} className="text-yellow-400 mt-0.5 flex-shrink-0" />
                }
                <div className="min-w-0">
                  <p className="text-xs text-white/70 leading-tight">{check.label}</p>
                  <p className="text-[10px] text-white/30">{check.detail}</p>
                </div>
              </div>
            ))}
          </div>
          <div className="mt-3 pt-3 border-t border-white/5">
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-white/30">5/7 checks OK</span>
              <span className="text-[10px] text-yellow-400 flex items-center gap-1">
                <AlertTriangle size={10} /> 2 varningar
              </span>
            </div>
          </div>
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
