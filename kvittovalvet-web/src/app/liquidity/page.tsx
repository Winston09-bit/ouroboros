'use client';
import { useState } from 'react';
import AppShell from '@/components/AppShell';
import StatCard from '@/components/StatCard';
import {
  TrendingDown, RefreshCw, AlertTriangle, CheckCircle2,
  ChevronDown, ChevronUp, X, CreditCard,
} from 'lucide-react';
import {
  AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer,
  ReferenceLine, CartesianGrid,
} from 'recharts';

/* ─── Demo data ─────────────────────────────────────────────────────── */
const FORECAST: {
  date: string; balance: number; inflow: number; outflow: number; confidence: number;
}[] = [
  { date: '1 maj',  balance: 250000, inflow: 0,      outflow: 0,      confidence: 99 },
  { date: '2 maj',  balance: 247000, inflow: 5000,   outflow: 8000,   confidence: 97 },
  { date: '3 maj',  balance: 244000, inflow: 3000,   outflow: 6000,   confidence: 95 },
  { date: '4 maj',  balance: 241000, inflow: 4000,   outflow: 7000,   confidence: 93 },
  { date: '5 maj',  balance: 237000, inflow: 2000,   outflow: 6000,   confidence: 91 },
  { date: '6 maj',  balance: 234000, inflow: 5000,   outflow: 8000,   confidence: 89 },
  { date: '7 maj',  balance: 230000, inflow: 3000,   outflow: 7000,   confidence: 88 },
  { date: '8 maj',  balance: 224000, inflow: 2000,   outflow: 8000,   confidence: 86 },
  { date: '9 maj',  balance: 195000, inflow: 1000,   outflow: 30000,  confidence: 84 },
  { date: '10 maj', balance: 162000, inflow: 2000,   outflow: 35000,  confidence: 82 },
  { date: '11 maj', balance: 138000, inflow: 6000,   outflow: 30000,  confidence: 80 },
  { date: '12 maj', balance: 82000,  inflow: 4000,   outflow: 60000,  confidence: 78 },
  { date: '13 maj', balance: 78000,  inflow: 8000,   outflow: 12000,  confidence: 76 },
  { date: '14 maj', balance: 88000,  inflow: 18000,  outflow: 8000,   confidence: 74 },
  { date: '15 maj', balance: 120000, inflow: 42000,  outflow: 10000,  confidence: 72 },
  { date: '16 maj', balance: 132000, inflow: 22000,  outflow: 10000,  confidence: 70 },
  { date: '17 maj', balance: 145000, inflow: 23000,  outflow: 10000,  confidence: 69 },
  { date: '18 maj', balance: 165000, inflow: 30000,  outflow: 10000,  confidence: 67 },
  { date: '19 maj', balance: 178000, inflow: 23000,  outflow: 10000,  confidence: 65 },
  { date: '20 maj', balance: 188000, inflow: 20000,  outflow: 10000,  confidence: 64 },
  { date: '21 maj', balance: 192000, inflow: 14000,  outflow: 10000,  confidence: 62 },
  { date: '22 maj', balance: 198000, inflow: 16000,  outflow: 10000,  confidence: 60 },
  { date: '23 maj', balance: 203000, inflow: 15000,  outflow: 10000,  confidence: 59 },
  { date: '24 maj', balance: 208000, inflow: 15000,  outflow: 10000,  confidence: 57 },
  { date: '25 maj', balance: 215000, inflow: 17000,  outflow: 10000,  confidence: 56 },
  { date: '26 maj', balance: 218000, inflow: 13000,  outflow: 10000,  confidence: 54 },
  { date: '27 maj', balance: 221000, inflow: 13000,  outflow: 10000,  confidence: 53 },
  { date: '28 maj', balance: 223000, inflow: 12000,  outflow: 10000,  confidence: 51 },
  { date: '29 maj', balance: 225000, inflow: 12000,  outflow: 10000,  confidence: 50 },
  { date: '30 maj', balance: 228000, inflow: 13000,  outflow: 10000,  confidence: 48 },
];

const fmt = (n: number) =>
  n.toLocaleString('sv-SE', { maximumFractionDigits: 0 }) + ' kr';

/* ─── Custom tooltip ─────────────────────────────────────────────────── */
interface TooltipProps {
  active?: boolean;
  payload?: Array<{ value: number; payload: typeof FORECAST[0] }>;
  label?: string;
}
const ChartTooltip = ({ active, payload, label }: TooltipProps) => {
  if (!active || !payload?.length) return null;
  const d = payload[0].payload;
  return (
    <div className="bg-[#1a1a2e] border border-white/10 rounded-xl px-4 py-3 text-xs shadow-2xl min-w-[170px]">
      <p className="text-white/60 font-medium mb-2">{label}</p>
      <p className="text-indigo-300 font-bold text-sm">{fmt(d.balance)}</p>
      {d.inflow > 0 && (
        <p className="text-green-400 mt-1">▲ Inflöden: {fmt(d.inflow)}</p>
      )}
      {d.outflow > 0 && (
        <p className="text-red-400">▼ Utflöden: {fmt(d.outflow)}</p>
      )}
      <p className="text-white/30 mt-1">Konfidans: {d.confidence}%</p>
    </div>
  );
};

/* ─── Confirm dialog ─────────────────────────────────────────────────── */
function ConfirmDialog({ onClose }: { onClose: () => void }) {
  const [accepted, setAccepted] = useState(false);
  if (accepted) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
        <div className="bg-[#12121c] border border-indigo-500/30 rounded-2xl p-8 max-w-sm w-full mx-4 text-center">
          <div className="w-14 h-14 rounded-full bg-indigo-500/20 flex items-center justify-center mx-auto mb-4">
            <CheckCircle2 size={28} className="text-indigo-400" />
          </div>
          <h3 className="text-white font-bold text-lg mb-2">Kredit accepterad</h3>
          <p className="text-white/50 text-sm mb-6">
            150,000 kr kommer att krediteras inom 1–2 bankdagar.
          </p>
          <button
            onClick={onClose}
            className="w-full py-2.5 rounded-xl bg-indigo-600 text-white text-sm font-medium hover:bg-indigo-500 transition-colors"
          >
            Stäng
          </button>
        </div>
      </div>
    );
  }
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
      <div className="bg-[#12121c] border border-white/10 rounded-2xl p-6 max-w-md w-full mx-4">
        <div className="flex items-center justify-between mb-5">
          <h3 className="text-white font-bold text-base flex items-center gap-2">
            <CreditCard size={16} className="text-indigo-400" />
            Bekräfta kreditansökan
          </h3>
          <button onClick={onClose} className="text-white/30 hover:text-white transition-colors">
            <X size={18} />
          </button>
        </div>
        <div className="space-y-3 mb-6">
          {[
            ['Belopp', '150,000 kr'],
            ['Löptid', '30 dagar'],
            ['Kostnad', '3,600 kr (0.08%/dag)'],
            ['Effektiv årsränta', '33.6%'],
            ['Totalt att återbetala', '153,600 kr'],
          ].map(([k, v]) => (
            <div key={k} className="flex justify-between text-sm border-b border-white/5 pb-2">
              <span className="text-white/50">{k}</span>
              <span className="text-white font-medium">{v}</span>
            </div>
          ))}
        </div>
        <p className="text-xs text-white/30 mb-5">
          Genom att acceptera godkänner du villkoren för kortfristig företagskredit.
          Medel betalas ut inom 1–2 bankdagar.
        </p>
        <div className="flex gap-3">
          <button
            onClick={onClose}
            className="flex-1 py-2.5 rounded-xl border border-white/10 text-white/50 text-sm hover:bg-white/5 transition-colors"
          >
            Avbryt
          </button>
          <button
            onClick={() => setAccepted(true)}
            className="flex-1 py-2.5 rounded-xl bg-indigo-600 text-white text-sm font-medium hover:bg-indigo-500 transition-colors"
          >
            Bekräfta kredit
          </button>
        </div>
      </div>
    </div>
  );
}

/* ─── Page ───────────────────────────────────────────────────────────── */
export default function LiquidityPage() {
  const [tableOpen, setTableOpen] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);

  return (
    <AppShell>
      {dialogOpen && <ConfirmDialog onClose={() => setDialogOpen(false)} />}

      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-xl font-semibold text-white">Likviditetsanalys</h1>
            <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-indigo-500/20 text-indigo-300 uppercase tracking-wider border border-indigo-500/30">
              AI-driven
            </span>
          </div>
          <p className="text-xs text-white/40 mt-0.5">30-dagars kassaflödsprognos med kreditbedömning</p>
        </div>
        <div className="flex items-center gap-3">
          <div className="text-right">
            <p className="text-xs text-white/40">Aktuellt saldo</p>
            <p className="text-2xl font-bold text-white tabular-nums">250,000 kr</p>
          </div>
          <button className="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-white/50 hover:text-white transition-all">
            <RefreshCw size={14} />
          </button>
        </div>
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-4 gap-4 mb-6">
        <StatCard
          label="Runway"
          value="~28 dagar"
          sub="utan kreditinjekt"
          accent="yellow"
          icon={<TrendingDown size={16} />}
        />
        <StatCard
          label="Min-saldo prognos"
          value="82,000 kr"
          sub="runt 12 maj"
          accent="red"
          icon={<AlertTriangle size={16} />}
        />
        <StatCard
          label="Recovery-konf."
          value="75%"
          sub="baserat på historik"
          accent="green"
          icon={<CheckCircle2 size={16} />}
        />
        <StatCard
          label="Kredit tillgänglig"
          value="150,000 kr"
          sub="gäller t.o.m. 27 maj"
          accent="indigo"
          icon={<CreditCard size={16} />}
        />
      </div>

      {/* Section 1: Cash Flow Forecast Chart */}
      <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5 mb-4">
        <h2 className="text-sm font-medium text-white/70 mb-4 uppercase tracking-wider">
          Kassaflödsprognos – 30 dagar
        </h2>
        <ResponsiveContainer width="100%" height={260}>
          <AreaChart data={FORECAST} margin={{ top: 10, right: 10, left: 10, bottom: 0 }}>
            <defs>
              <linearGradient id="balanceGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%"  stopColor="#6366f1" stopOpacity={0.35} />
                <stop offset="95%" stopColor="#6366f1" stopOpacity={0.02} />
              </linearGradient>
              <linearGradient id="safeGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%"  stopColor="#22c55e" stopOpacity={0.10} />
                <stop offset="100%" stopColor="#22c55e" stopOpacity={0.02} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.04)" />
            <XAxis
              dataKey="date"
              tick={{ fill: 'rgba(255,255,255,0.25)', fontSize: 10 }}
              axisLine={false}
              tickLine={false}
              interval={4}
            />
            <YAxis
              tick={{ fill: 'rgba(255,255,255,0.25)', fontSize: 10 }}
              axisLine={false}
              tickLine={false}
              tickFormatter={(v) => `${(v / 1000).toFixed(0)}k`}
              domain={[0, 300000]}
            />
            <Tooltip content={<ChartTooltip />} cursor={{ stroke: 'rgba(99,102,241,0.3)', strokeWidth: 1 }} />
            {/* Safe zone reference */}
            <ReferenceLine
              y={150000}
              stroke="#22c55e"
              strokeDasharray="5 5"
              strokeOpacity={0.4}
              label={{ value: 'Safe zone', fill: '#22c55e', fontSize: 10, opacity: 0.6, position: 'insideTopRight' }}
            />
            {/* Critical threshold */}
            <ReferenceLine
              y={50000}
              stroke="#ef4444"
              strokeDasharray="5 5"
              strokeOpacity={0.6}
              label={{ value: 'Kritisk gräns', fill: '#ef4444', fontSize: 10, opacity: 0.7, position: 'insideTopRight' }}
            />
            <Area
              type="monotone"
              dataKey="balance"
              stroke="#6366f1"
              strokeWidth={2}
              fill="url(#balanceGrad)"
              dot={false}
              activeDot={{ r: 4, fill: '#6366f1', stroke: '#fff', strokeWidth: 1.5 }}
            />
          </AreaChart>
        </ResponsiveContainer>
        <div className="flex items-center gap-5 mt-2 text-[10px] text-white/30">
          <span className="flex items-center gap-1.5">
            <span className="w-6 h-0.5 bg-indigo-500 rounded" />
            Prognostiserat saldo
          </span>
          <span className="flex items-center gap-1.5">
            <span className="w-6 h-0.5 bg-green-500 opacity-50 rounded" style={{ borderTop: '2px dashed #22c55e' }} />
            Safe zone (150k)
          </span>
          <span className="flex items-center gap-1.5">
            <span className="w-6 h-0.5 bg-red-500 opacity-60 rounded" style={{ borderTop: '2px dashed #ef4444' }} />
            Kritisk gräns (50k)
          </span>
        </div>
      </div>

      {/* Section 2: Likviditetsrisk */}
      <div className="rounded-xl border border-yellow-500/20 bg-yellow-500/5 p-5 mb-4">
        <div className="flex items-start justify-between mb-4">
          <h2 className="text-sm font-medium text-yellow-300 uppercase tracking-wider flex items-center gap-2">
            <AlertTriangle size={14} className="text-yellow-400" />
            Likviditetsrisk-indikator
          </h2>
          <span className="text-xs font-bold px-2.5 py-1 rounded-full bg-yellow-500/20 text-yellow-300 border border-yellow-500/30">
            ⚠️ WARNING
          </span>
        </div>
        <div className="grid grid-cols-3 gap-4">
          <div className="rounded-lg bg-black/20 border border-white/5 p-4">
            <p className="text-[10px] text-white/40 uppercase tracking-wider mb-1">Runway</p>
            <p className="text-xl font-bold text-yellow-300">~28 dagar</p>
            <p className="text-[10px] text-white/30 mt-0.5">utan externt kapital</p>
          </div>
          <div className="rounded-lg bg-black/20 border border-white/5 p-4">
            <p className="text-[10px] text-white/40 uppercase tracking-wider mb-1">Dip-fönster</p>
            <p className="text-base font-bold text-yellow-200">9 maj – 19 maj</p>
            <p className="text-xs text-red-400 mt-0.5 font-mono">djup: −82,000 kr</p>
          </div>
          <div className="rounded-lg bg-black/20 border border-white/5 p-4">
            <p className="text-[10px] text-white/40 uppercase tracking-wider mb-1">Recovery-konfidens</p>
            <p className="text-xl font-bold text-green-400">75%</p>
            <div className="w-full h-1.5 bg-white/10 rounded-full mt-2 overflow-hidden">
              <div className="h-full bg-green-500 rounded-full transition-all" style={{ width: '75%' }} />
            </div>
          </div>
        </div>
      </div>

      {/* Section 3: Krediterbjudande */}
      <div className="rounded-xl border border-indigo-500/30 bg-gradient-to-br from-indigo-500/10 via-indigo-500/5 to-transparent p-5 mb-4">
        <div className="flex items-start gap-3 mb-5">
          <div className="w-8 h-8 rounded-lg bg-indigo-500/20 flex items-center justify-center flex-shrink-0">
            <span className="text-base">💡</span>
          </div>
          <div>
            <h2 className="text-sm font-bold text-white">Proaktivt krediterbjudande</h2>
            <p className="text-xs text-white/40 mt-0.5">
              Systemet har detekterat en tillfällig likviditetssvacka med stark återhämtningspotential
            </p>
          </div>
        </div>

        {/* Credit card */}
        <div className="rounded-xl border border-indigo-500/30 bg-gradient-to-br from-indigo-600/30 to-indigo-900/30 p-5 mb-4">
          <div className="flex items-center justify-between mb-4">
            <CreditCard size={18} className="text-indigo-300" />
            <span className="text-[10px] text-indigo-400 font-medium px-2 py-0.5 rounded-full bg-indigo-500/10 border border-indigo-500/20">
              Grade B
            </span>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <p className="text-[10px] text-white/40 uppercase tracking-wider">Erbjudet belopp</p>
              <p className="text-2xl font-bold text-white mt-0.5">150,000 kr</p>
            </div>
            <div>
              <p className="text-[10px] text-white/40 uppercase tracking-wider">Kostnad</p>
              <p className="text-lg font-bold text-indigo-300 mt-0.5">3,600 kr</p>
              <p className="text-[10px] text-white/30">0.08%/dag</p>
            </div>
            <div>
              <p className="text-[10px] text-white/40 uppercase tracking-wider">Effektiv årsränta</p>
              <p className="text-base font-bold text-white/70 mt-0.5">33.6%</p>
            </div>
            <div>
              <p className="text-[10px] text-white/40 uppercase tracking-wider">Löptid</p>
              <p className="text-base font-bold text-white/70 mt-0.5">30 dagar</p>
            </div>
          </div>
          <div className="mt-4 pt-3 border-t border-indigo-500/20">
            <p className="text-[10px] text-indigo-400/70">
              Erbjudandet gäller till: <span className="font-mono font-bold text-indigo-300">2026-05-27 00:00</span>
            </p>
          </div>
        </div>

        {/* Reasoning */}
        <div className="rounded-lg bg-black/20 border border-white/5 p-3 mb-4 text-xs text-white/50 italic">
          "Grade B – Stabila återkommande intäkter detekterade. Liknande svacka hanterades framgångsrikt Q1 2026."
        </div>

        {/* Signals */}
        <div className="grid grid-cols-2 gap-3 mb-5">
          <div>
            <p className="text-[10px] text-green-400 uppercase tracking-wider mb-2 font-medium">Positiva signaler</p>
            <ul className="space-y-1.5">
              {[
                'Återkommande kundbetalningar väntas under perioden',
                'Historisk seasonal recovery i juni',
                '85% av kostnader är förutsägbara',
              ].map((s) => (
                <li key={s} className="flex items-start gap-1.5 text-xs text-white/60">
                  <CheckCircle2 size={12} className="text-green-400 mt-0.5 flex-shrink-0" />
                  {s}
                </li>
              ))}
            </ul>
          </div>
          <div>
            <p className="text-[10px] text-yellow-400 uppercase tracking-wider mb-2 font-medium">Riskfaktorer</p>
            <ul className="space-y-1.5">
              {[
                '3 månaders data (lågt underlag)',
                'Hög månadsvariation i intäkter',
              ].map((s) => (
                <li key={s} className="flex items-start gap-1.5 text-xs text-white/60">
                  <AlertTriangle size={12} className="text-yellow-400 mt-0.5 flex-shrink-0" />
                  {s}
                </li>
              ))}
            </ul>
          </div>
        </div>

        {/* CTA */}
        <div className="flex gap-3">
          <button
            onClick={() => setDialogOpen(true)}
            className="flex-1 py-3 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-sm transition-colors flex items-center justify-center gap-2"
          >
            <CreditCard size={15} />
            Acceptera kredit
          </button>
          <button className="px-6 py-3 rounded-xl border border-white/10 text-white/50 hover:text-white hover:bg-white/5 text-sm transition-colors">
            Avböj
          </button>
        </div>
      </div>

      {/* Section 4: Collapsible detail table */}
      <div className="rounded-xl border border-white/5 bg-white/[0.02] overflow-hidden">
        <button
          className="w-full flex items-center justify-between px-5 py-4 hover:bg-white/[0.02] transition-colors"
          onClick={() => setTableOpen((v) => !v)}
        >
          <h2 className="text-sm font-medium text-white/70 uppercase tracking-wider">
            Detaljerade dagliga prognoser
          </h2>
          {tableOpen
            ? <ChevronUp size={16} className="text-white/40" />
            : <ChevronDown size={16} className="text-white/40" />
          }
        </button>
        {tableOpen && (
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-t border-white/5">
                  {['Datum', 'Balans', 'Inflöden', 'Utflöden', 'Konfidens'].map((h) => (
                    <th
                      key={h}
                      className="text-left px-5 py-2 text-white/30 uppercase tracking-wider font-medium"
                    >
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {FORECAST.map((row) => {
                  const isLow  = row.balance < 100000;
                  const isVlow = row.balance < 100000 && row.balance < 90000;
                  return (
                    <tr
                      key={row.date}
                      className={`border-t border-white/5 transition-colors hover:bg-white/[0.02] ${
                        isVlow ? 'bg-red-500/5' : isLow ? 'bg-yellow-500/5' : ''
                      }`}
                    >
                      <td className="px-5 py-2.5 text-white/70 font-medium">{row.date}</td>
                      <td className={`px-5 py-2.5 font-mono font-bold ${
                        isVlow ? 'text-red-400' : isLow ? 'text-yellow-400' : 'text-white'
                      }`}>
                        {fmt(row.balance)}
                      </td>
                      <td className="px-5 py-2.5 text-green-400 font-mono">
                        {row.inflow > 0 ? `+${fmt(row.inflow)}` : '–'}
                      </td>
                      <td className="px-5 py-2.5 text-red-400 font-mono">
                        {row.outflow > 0 ? `−${fmt(row.outflow)}` : '–'}
                      </td>
                      <td className="px-5 py-2.5">
                        <div className="flex items-center gap-2">
                          <div className="flex-1 h-1 bg-white/10 rounded-full overflow-hidden max-w-[60px]">
                            <div
                              className="h-full bg-indigo-500 rounded-full"
                              style={{ width: `${row.confidence}%` }}
                            />
                          </div>
                          <span className="text-white/30">{row.confidence}%</span>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </AppShell>
  );
}
