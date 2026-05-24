'use client';
import AppShell from '@/components/AppShell';
import { FileCheck, FileMinus, Send, Mail, MessageSquare, FileWarning } from 'lucide-react';

const evidence = [
  {
    id: 'txn-001',
    merchant: 'Elgiganten AB',
    amount: '12 800 kr',
    date: '2026-05-23',
    state: 'MISSING',
    steps: [
      { label: 'API Retrieval',    done: true,  ts: '2026-05-23 14:35' },
      { label: 'Peppol Request',   done: true,  ts: '2026-05-23 14:36' },
      { label: 'AI-mail',          done: true,  ts: '2026-05-23 14:37' },
      { label: 'SMS Påminnelse',   done: false, ts: null },
      { label: 'Rek. brev',        done: false, ts: null },
      { label: 'Juridisk export',  done: false, ts: null },
    ],
  },
  {
    id: 'txn-002',
    merchant: 'Amazon EU',
    amount: '3 200 kr',
    date: '2026-05-21',
    state: 'REQUESTED',
    steps: [
      { label: 'API Retrieval',    done: true,  ts: '2026-05-21 11:22' },
      { label: 'Peppol Request',   done: true,  ts: '2026-05-21 11:23' },
      { label: 'AI-mail',          done: true,  ts: '2026-05-21 11:24' },
      { label: 'SMS Påminnelse',   done: true,  ts: '2026-05-22 09:00' },
      { label: 'Rek. brev',        done: false, ts: null },
      { label: 'Juridisk export',  done: false, ts: null },
    ],
  },
];

const stateConfig: Record<string, { label: string; color: string; icon: React.ReactNode }> = {
  MISSING:   { label: 'Saknat underlag', color: 'text-yellow-400 bg-yellow-500/10 border-yellow-500/20', icon: <FileMinus size={14} /> },
  REQUESTED: { label: 'Begärt',          color: 'text-blue-400 bg-blue-500/10 border-blue-500/20',       icon: <Send size={14} /> },
  FOUND:     { label: 'Hittat',          color: 'text-green-400 bg-green-500/10 border-green-500/20',    icon: <FileCheck size={14} /> },
  ESCALATED: { label: 'Eskalerat',       color: 'text-red-400 bg-red-500/10 border-red-500/20',          icon: <FileWarning size={14} /> },
};

const stepIcons: Record<string, React.ReactNode> = {
  'API Retrieval':   <FileCheck size={12} />,
  'Peppol Request':  <Send size={12} />,
  'AI-mail':         <Mail size={12} />,
  'SMS Påminnelse':  <MessageSquare size={12} />,
  'Rek. brev':       <Mail size={12} />,
  'Juridisk export': <FileWarning size={12} />,
};

export default function Evidence() {
  return (
    <AppShell>
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-white">Underlag</h1>
        <p className="text-xs text-white/40 mt-0.5">Saknade underlag och retrieval-status</p>
      </div>

      <div className="space-y-4">
        {evidence.map(ev => {
          const sc = stateConfig[ev.state];
          return (
            <div key={ev.id} className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
              {/* Header */}
              <div className="flex items-start justify-between mb-4">
                <div>
                  <p className="font-medium text-white">{ev.merchant}</p>
                  <p className="text-xs text-white/40 mt-0.5">{ev.amount} · {ev.date}</p>
                </div>
                <span className={`flex items-center gap-1.5 px-2 py-1 rounded border text-xs font-medium ${sc.color}`}>
                  {sc.icon}{sc.label}
                </span>
              </div>

              {/* Retrieval chain */}
              <div className="flex items-center gap-1">
                {ev.steps.map((step, i) => (
                  <div key={step.label} className="flex items-center gap-1 flex-1">
                    <div className={`group relative flex flex-col items-center gap-1 flex-1`}>
                      <div className={`w-7 h-7 rounded-full flex items-center justify-center text-[10px] transition-colors ${
                        step.done
                          ? 'bg-green-500/20 text-green-400 border border-green-500/30'
                          : 'bg-white/5 text-white/20 border border-white/10'
                      }`}>
                        {stepIcons[step.label]}
                      </div>
                      <p className={`text-[9px] text-center leading-tight ${step.done ? 'text-white/50' : 'text-white/20'}`}>
                        {step.label}
                      </p>
                      {step.ts && (
                        <p className="text-[9px] text-white/20">{step.ts.split(' ')[1]}</p>
                      )}
                    </div>
                    {i < ev.steps.length - 1 && (
                      <div className={`w-4 h-px flex-shrink-0 mb-4 ${step.done ? 'bg-green-500/30' : 'bg-white/10'}`} />
                    )}
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </AppShell>
  );
}
