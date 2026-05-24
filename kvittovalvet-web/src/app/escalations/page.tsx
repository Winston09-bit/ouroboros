'use client';
import AppShell from '@/components/AppShell';
import { Zap, ChevronRight } from 'lucide-react';

const escalations = [
  {
    id: 'ESC-001',
    merchant: 'Elgiganten AB',
    amount: '12 800 kr',
    started: '2026-05-23',
    currentStep: 3,
    maxStep: 7,
    nextAction: 'SMS-påminnelse schemalagd',
    nextAt: '2026-05-25 09:00',
  },
  {
    id: 'ESC-002',
    merchant: 'Amazon EU',
    amount: '3 200 kr',
    started: '2026-05-21',
    currentStep: 4,
    maxStep: 7,
    nextAction: 'Rekommenderat brev förbereds',
    nextAt: '2026-05-26 10:00',
  },
];

export default function Escalations() {
  return (
    <AppShell>
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-white">Eskaleringar</h1>
        <p className="text-xs text-white/40 mt-0.5">Autonoma eskaleringskedjor för saknade underlag</p>
      </div>

      <div className="space-y-4">
        {escalations.map(e => (
          <div key={e.id} className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
            <div className="flex items-start justify-between mb-4">
              <div>
                <div className="flex items-center gap-2">
                  <Zap size={14} className="text-orange-400" />
                  <p className="font-medium text-white">{e.merchant}</p>
                  <span className="text-xs text-white/30 font-mono">{e.id}</span>
                </div>
                <p className="text-xs text-white/40 mt-0.5">{e.amount} · Startad {e.started}</p>
              </div>
              <span className="text-xs px-2 py-1 rounded border border-orange-500/20 bg-orange-500/10 text-orange-400">
                Steg {e.currentStep}/{e.maxStep}
              </span>
            </div>

            {/* Progress bar */}
            <div className="h-1.5 bg-white/5 rounded-full mb-4 overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-orange-500 to-red-500 rounded-full"
                style={{ width: `${(e.currentStep / e.maxStep) * 100}%` }}
              />
            </div>

            {/* Next action */}
            <div className="flex items-center justify-between text-xs">
              <div className="flex items-center gap-2 text-white/50">
                <ChevronRight size={12} className="text-orange-400" />
                <span>{e.nextAction}</span>
              </div>
              <span className="text-white/25">{e.nextAt}</span>
            </div>
          </div>
        ))}
      </div>
    </AppShell>
  );
}
