'use client';
import AppShell from '@/components/AppShell';
import { CheckCircle, XCircle, ExternalLink } from 'lucide-react';

const integrations = [
  { id: 'fortnox',        label: 'Fortnox',         type: 'ERP',     connected: true,  detail: 'API v3 · OAuth2' },
  { id: 'visma',          label: 'Visma eEkonomi',   type: 'ERP',     connected: false, detail: 'Ej konfigurerad' },
  { id: 'tink',           label: 'Tink',             type: 'Bank',    connected: false, detail: 'PSD2 aggregator' },
  { id: 'enable-banking', label: 'Enable Banking',   type: 'Bank',    connected: false, detail: 'Nordea, SEB...' },
  { id: 'revolut',        label: 'Revolut Business', type: 'Bank',    connected: false, detail: 'Pending OAuth' },
  { id: 'peppol',         label: 'Peppol',           type: 'E-faktura',connected: false,detail: 'Accesspunkt saknas' },
  { id: 'kivra',          label: 'Kivra',            type: 'Digital', connected: false, detail: 'API ej konfigurerad' },
];

export default function Settings() {
  return (
    <AppShell>
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-white">Inställningar</h1>
        <p className="text-xs text-white/40 mt-0.5">Integrationer och anslutningar</p>
      </div>

      <div className="rounded-xl border border-white/5 overflow-hidden">
        <div className="px-5 py-3 border-b border-white/5 bg-white/[0.02]">
          <p className="text-xs font-medium text-white/50 uppercase tracking-wider">Anslutningar</p>
        </div>
        <div className="divide-y divide-white/5">
          {integrations.map(intg => (
            <div key={intg.id} className="flex items-center justify-between px-5 py-4 hover:bg-white/[0.02] transition-colors">
              <div className="flex items-center gap-3">
                {intg.connected
                  ? <CheckCircle size={15} className="text-green-400" />
                  : <XCircle size={15} className="text-white/20" />}
                <div>
                  <p className={`text-sm font-medium ${intg.connected ? 'text-white' : 'text-white/50'}`}>{intg.label}</p>
                  <p className="text-xs text-white/25">{intg.detail}</p>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <span className="text-[10px] px-2 py-0.5 rounded bg-white/5 text-white/30 uppercase">{intg.type}</span>
                <button className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs transition-all ${
                  intg.connected
                    ? 'bg-green-500/10 text-green-400 hover:bg-green-500/20 border border-green-500/20'
                    : 'bg-indigo-500/10 text-indigo-400 hover:bg-indigo-500/20 border border-indigo-500/20'
                }`}>
                  <ExternalLink size={11} />
                  {intg.connected ? 'Hantera' : 'Anslut'}
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </AppShell>
  );
}
