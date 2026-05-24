'use client';
import { useEffect, useState } from 'react';
import AppShell from '@/components/AppShell';
import { api } from '@/lib/api';
import { CheckCircle, AlertCircle, Clock, Search } from 'lucide-react';

interface Transaction {
  id: string;
  amount: string;
  currency: string;
  timestamp: string;
  merchant?: { name: string };
  status: 'matched' | 'unmatched' | 'pending' | 'error';
  confidence: number;
  source: string;
}

const statusIcon = {
  matched:   <CheckCircle size={14} className="text-green-400" />,
  unmatched: <AlertCircle size={14} className="text-yellow-400" />,
  pending:   <Clock size={14} className="text-blue-400" />,
  error:     <AlertCircle size={14} className="text-red-400" />,
};

const statusLabel = {
  matched:   'Matchad',
  unmatched: 'Saknar underlag',
  pending:   'Väntar',
  error:     'Fel',
};

const DEMO: Transaction[] = [
  { id: '1', amount: '4 590.00', currency: 'SEK', timestamp: '2026-05-24T08:12:00Z', merchant: { name: 'ICA Maxi Barkarby' }, status: 'matched',   confidence: 0.97, source: 'nordea' },
  { id: '2', amount: '12 800.00', currency: 'SEK', timestamp: '2026-05-23T14:33:00Z', merchant: { name: 'Elgiganten AB' },     status: 'unmatched', confidence: 0.00, source: 'revolut' },
  { id: '3', amount: '890.00',  currency: 'SEK', timestamp: '2026-05-23T09:05:00Z', merchant: { name: 'Clas Ohlson' },        status: 'matched',   confidence: 0.91, source: 'nordea' },
  { id: '4', amount: '56 000.00', currency: 'SEK', timestamp: '2026-05-22T16:00:00Z', merchant: { name: 'TryggBil AB' },      status: 'matched',   confidence: 0.99, source: 'fortnox' },
  { id: '5', amount: '3 200.00', currency: 'SEK', timestamp: '2026-05-21T11:20:00Z', merchant: { name: 'Amazon EU' },         status: 'unmatched', confidence: 0.00, source: 'revolut' },
];

export default function Transactions() {
  const [txs, setTxs]     = useState<Transaction[]>(DEMO);
  const [q, setQ]         = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.transactions()
      .then(data => { if (Array.isArray(data) && data.length) setTxs(data); })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const filtered = txs.filter(t =>
    !q || t.merchant?.name?.toLowerCase().includes(q.toLowerCase())
  );

  return (
    <AppShell>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-white">Transaktioner</h1>
          <p className="text-xs text-white/40 mt-0.5">{txs.length} transaktioner indexerade</p>
        </div>
        <div className="relative">
          <Search size={13} className="absolute left-3 top-1/2 -translate-y-1/2 text-white/30" />
          <input
            value={q}
            onChange={e => setQ(e.target.value)}
            placeholder="Sök merchant…"
            className="pl-8 pr-4 py-2 text-sm bg-white/5 border border-white/10 rounded-lg text-white placeholder-white/30 focus:outline-none focus:border-indigo-500/50 w-52"
          />
        </div>
      </div>

      <div className="rounded-xl border border-white/5 overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-white/5 bg-white/[0.02]">
              <th className="text-left px-4 py-3 text-xs font-medium text-white/40 uppercase tracking-wider">Status</th>
              <th className="text-left px-4 py-3 text-xs font-medium text-white/40 uppercase tracking-wider">Merchant</th>
              <th className="text-right px-4 py-3 text-xs font-medium text-white/40 uppercase tracking-wider">Belopp</th>
              <th className="text-left px-4 py-3 text-xs font-medium text-white/40 uppercase tracking-wider">Källa</th>
              <th className="text-right px-4 py-3 text-xs font-medium text-white/40 uppercase tracking-wider">Säkerhet</th>
              <th className="text-left px-4 py-3 text-xs font-medium text-white/40 uppercase tracking-wider">Tid</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-white/5">
            {filtered.map(t => (
              <tr key={t.id} className="hover:bg-white/[0.02] transition-colors">
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    {statusIcon[t.status]}
                    <span className={`text-xs ${
                      t.status === 'matched' ? 'text-green-400' :
                      t.status === 'unmatched' ? 'text-yellow-400' : 'text-white/50'
                    }`}>{statusLabel[t.status]}</span>
                  </div>
                </td>
                <td className="px-4 py-3 text-white/80 font-medium">{t.merchant?.name ?? '–'}</td>
                <td className="px-4 py-3 text-right font-mono text-white">{t.amount} {t.currency}</td>
                <td className="px-4 py-3">
                  <span className="px-2 py-0.5 rounded text-[10px] font-medium bg-indigo-500/10 text-indigo-400 uppercase">{t.source}</span>
                </td>
                <td className="px-4 py-3 text-right">
                  {t.confidence > 0 ? (
                    <span className="text-xs text-green-400">{Math.round(t.confidence * 100)}%</span>
                  ) : (
                    <span className="text-xs text-white/20">–</span>
                  )}
                </td>
                <td className="px-4 py-3 text-xs text-white/30">
                  {new Date(t.timestamp).toLocaleDateString('sv-SE')}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </AppShell>
  );
}
