'use client';
import { useState } from 'react';
import AppShell from '@/components/AppShell';
import {
  FileCode2, ShieldCheck, CheckCircle2, XCircle,
  ChevronRight, Hash, Clock, Key, Fingerprint,
} from 'lucide-react';

const VRF_EXAMPLE = {
  vrf_version: "1.0",
  receipt_id: "vrf_01hwn3k8x2q5v6p9b4m7t3n1",
  issued_at: "2026-05-24T12:34:56Z",
  merchant: {
    merchant_id: "ICA",
    display_name: "ICA Gruppen",
    org_number: "556015-0875",
  },
  transaction: {
    amount: 489.50,
    currency: "SEK",
    date: "2026-05-24",
    reference: "4729-A2F3",
    payment_method: "card",
    last4: "4242",
  },
  line_items: [
    { description: "Oatly Havredryck 1L", quantity: 2, unit_price: 34.90, total: 69.80, vat_rate: 0.12 },
    { description: "Ekologisk Kycklingfilé 600g", quantity: 1, unit_price: 89.90, total: 89.90, vat_rate: 0.12 },
    { description: "San Pellegrino 6-pack", quantity: 3, unit_price: 49.90, total: 149.70, vat_rate: 0.12 },
    { description: "Ibuprofen 400mg 20-pack", quantity: 2, unit_price: 90.05, total: 180.10, vat_rate: 0.25 },
  ],
  vat_summary: [
    { rate: 0.12, taxable_amount: 309.40, vat_amount: 37.13 },
    { rate: 0.25, taxable_amount: 144.08, vat_amount: 36.02 },
  ],
  chain_of_custody: [
    {
      step: 1,
      actor: "ICA API Gateway",
      timestamp: "2026-05-24T12:34:58Z",
      action: "RECEIPT_CREATED",
      hash: "sha256:a3f9c2b1e8d7f4...",
    },
    {
      step: 2,
      actor: "Kvittovalvet Ingestion",
      timestamp: "2026-05-24T12:35:02Z",
      action: "RECEIPT_INGESTED",
      hash: "sha256:b7d4e1a9f2c8...",
    },
    {
      step: 3,
      actor: "Kvittovalvet Verification",
      timestamp: "2026-05-24T12:35:04Z",
      action: "RECEIPT_VERIFIED",
      hash: "sha256:c9e2f7d3a1b4...",
    },
  ],
  signature: {
    algorithm: "ES256",
    issuer: "kvittovalvet.landvex.se",
    key_id: "kv-signing-2026-q2",
    value: "eyJhbGciOiJFUzI1NiIsImtpZCI6Imt2LXNpZ25pbmctMjAyNi1xMiJ9...",
  },
};

type VerifyState = 'idle' | 'loading' | 'valid' | 'invalid';

function JsonLine({ line, depth = 0 }: { line: string; depth?: number }) {
  const isKey = line.includes('":');
  const isString = line.trim().startsWith('"') && !isKey;
  const isNumber = /^\s*[\d.]+/.test(line.trim());
  const isBool = /^\s*(true|false)/.test(line.trim());
  const isBracket = /^[{}\[\],]/.test(line.trim());

  return (
    <span className={
      isKey ? 'text-blue-300' :
      isString ? 'text-green-300' :
      isNumber ? 'text-orange-300' :
      isBool ? 'text-purple-300' :
      'text-white/50'
    }>{line}</span>
  );
}

function PrettyJson({ data }: { data: unknown }) {
  const lines = JSON.stringify(data, null, 2).split('\n');
  return (
    <pre className="text-xs font-mono leading-5 overflow-auto">
      {lines.map((line, i) => (
        <span key={i} className="block">
          <JsonLine line={line} />
        </span>
      ))}
    </pre>
  );
}

export default function ReceiptFormatPage() {
  const [verifyState, setVerifyState] = useState<VerifyState>('idle');
  const [activeTab, setActiveTab] = useState<'vrf' | 'chain' | 'signature'>('vrf');

  const handleVerify = async () => {
    setVerifyState('loading');
    await new Promise(r => setTimeout(r, 1200));
    setVerifyState('valid');
  };

  return (
    <AppShell>
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-white flex items-center gap-2">
            <FileCode2 size={20} className="text-indigo-400" />
            VRF – Verified Receipt Format
          </h1>
          <p className="text-xs text-white/40 mt-0.5">Kvittovalvets standardformat för verifierbara underlag</p>
        </div>
        <span className="text-[10px] px-2.5 py-1 rounded-full bg-indigo-500/20 text-indigo-300 border border-indigo-500/30 font-mono">
          v1.0 · DRAFT
        </span>
      </div>

      {/* Info cards */}
      <div className="grid grid-cols-4 gap-3 mb-6">
        {[
          { icon: <Hash size={14} />, label: 'Schema', value: 'JSON+Signature', color: 'text-indigo-400' },
          { icon: <Key size={14} />, label: 'Signatur', value: 'ES256 (ECDSA)', color: 'text-green-400' },
          { icon: <Fingerprint size={14} />, label: 'Chain', value: 'SHA-256 hash', color: 'text-yellow-400' },
          { icon: <Clock size={14} />, label: 'Timestamp', value: 'ISO 8601 UTC', color: 'text-purple-400' },
        ].map(item => (
          <div key={item.label} className="rounded-xl border border-white/5 bg-white/[0.02] p-3">
            <div className={`flex items-center gap-1.5 mb-1.5 ${item.color}`}>
              {item.icon}
              <span className="text-[10px] uppercase tracking-wider font-medium">{item.label}</span>
            </div>
            <p className="text-sm font-semibold text-white font-mono">{item.value}</p>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* Left: JSON viewer */}
        <div className="rounded-xl border border-white/5 bg-white/[0.02] overflow-hidden">
          {/* Tabs */}
          <div className="flex border-b border-white/5 bg-black/20">
            {(['vrf', 'chain', 'signature'] as const).map(tab => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`px-4 py-2.5 text-xs font-medium transition-all ${
                  activeTab === tab
                    ? 'text-indigo-300 border-b-2 border-indigo-500'
                    : 'text-white/40 hover:text-white'
                }`}
              >
                {tab === 'vrf' ? 'VRF Payload' : tab === 'chain' ? 'Chain of Custody' : 'Signatur'}
              </button>
            ))}
          </div>
          <div className="p-4 max-h-96 overflow-y-auto scrollbar-thin">
            {activeTab === 'vrf' && <PrettyJson data={{
              vrf_version: VRF_EXAMPLE.vrf_version,
              receipt_id: VRF_EXAMPLE.receipt_id,
              issued_at: VRF_EXAMPLE.issued_at,
              merchant: VRF_EXAMPLE.merchant,
              transaction: VRF_EXAMPLE.transaction,
              line_items: VRF_EXAMPLE.line_items,
              vat_summary: VRF_EXAMPLE.vat_summary,
            }} />}
            {activeTab === 'chain' && <PrettyJson data={{ chain_of_custody: VRF_EXAMPLE.chain_of_custody }} />}
            {activeTab === 'signature' && <PrettyJson data={{ signature: VRF_EXAMPLE.signature }} />}
          </div>
        </div>

        {/* Right: Chain visualizer + Verify */}
        <div className="flex flex-col gap-4">
          {/* Chain visualization */}
          <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
            <h2 className="text-sm font-medium text-white/70 mb-4 uppercase tracking-wider flex items-center gap-2">
              <ShieldCheck size={14} className="text-green-400" />
              Signaturkedja
            </h2>
            <div className="space-y-3">
              {VRF_EXAMPLE.chain_of_custody.map((step, i) => (
                <div key={step.step} className="flex gap-3">
                  <div className="flex flex-col items-center">
                    <div className="w-6 h-6 rounded-full bg-green-500/20 border border-green-500/40 flex items-center justify-center text-[10px] font-bold text-green-400 flex-shrink-0">
                      {step.step}
                    </div>
                    {i < VRF_EXAMPLE.chain_of_custody.length - 1 && (
                      <div className="w-px flex-1 bg-white/10 my-1" />
                    )}
                  </div>
                  <div className="pb-3">
                    <div className="flex items-center gap-2 mb-0.5">
                      <span className="text-xs text-white font-medium">{step.actor}</span>
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-green-500/10 text-green-400 border border-green-500/20 font-mono">
                        {step.action}
                      </span>
                    </div>
                    <p className="text-[10px] text-white/30 font-mono">{step.hash}</p>
                    <p className="text-[10px] text-white/20 mt-0.5">{step.timestamp}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Verify button */}
          <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
            <h2 className="text-sm font-medium text-white/70 mb-3 uppercase tracking-wider">Verifiera kvitto</h2>
            <p className="text-xs text-white/40 mb-4">
              Verifierar signatur (ES256), hash-kedja och merchant-identitet mot Kvittovalvets PKI.
            </p>

            {verifyState === 'valid' && (
              <div className="flex items-center gap-3 p-3 rounded-lg bg-green-500/10 border border-green-500/20 mb-4">
                <CheckCircle2 size={18} className="text-green-400 flex-shrink-0" />
                <div>
                  <p className="text-sm font-medium text-green-400">Signatur giltig</p>
                  <p className="text-[10px] text-green-400/60">Hash-kedja integritetsverifierad · Utfärdare betrodd</p>
                </div>
              </div>
            )}
            {verifyState === 'invalid' && (
              <div className="flex items-center gap-3 p-3 rounded-lg bg-red-500/10 border border-red-500/20 mb-4">
                <XCircle size={18} className="text-red-400 flex-shrink-0" />
                <div>
                  <p className="text-sm font-medium text-red-400">Verifiering misslyckades</p>
                  <p className="text-[10px] text-red-400/60">Ogiltig signatur eller manipulerat underlag</p>
                </div>
              </div>
            )}

            <button
              onClick={handleVerify}
              disabled={verifyState === 'loading'}
              className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white text-sm font-medium transition-all"
            >
              {verifyState === 'loading' ? (
                <>
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Verifierar…
                </>
              ) : (
                <>
                  <ShieldCheck size={14} />
                  POST /vrf/verify
                  <ChevronRight size={14} />
                </>
              )}
            </button>
          </div>

          {/* Spec link */}
          <div className="rounded-xl border border-white/5 bg-white/[0.02] p-4">
            <p className="text-xs text-white/40 leading-relaxed">
              VRF är Kvittovalvets öppna standard för digitala underlag som uppfyller{' '}
              <span className="text-white/60">FAR (Föreningen Auktoriserade Revisorer)</span>{' '}
              och <span className="text-white/60">Bokföringslagen §5</span> krav på verifierbara affärshändelser.
            </p>
          </div>
        </div>
      </div>
    </AppShell>
  );
}
