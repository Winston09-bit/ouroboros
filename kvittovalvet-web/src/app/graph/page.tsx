'use client';
import { useState } from 'react';
import AppShell from '@/components/AppShell';
import StatCard from '@/components/StatCard';
import {
  GitGraph, Network, GitMerge, AlertTriangle,
  TrendingUp, RefreshCw, ChevronRight,
} from 'lucide-react';

interface GraphNode {
  id: string;
  label: string;
  type: 'merchant' | 'transaction' | 'receipt' | 'company';
  degree: number;
}

interface GraphEdge {
  source: string;
  target: string;
  weight: number;
  type: string;
}

interface DuplicateCandidate {
  id: string;
  names: string[];
  confidence: number;
  reason: string;
}

const MOCK_NODES: GraphNode[] = [
  { id: 'ICA',        label: 'ICA Gruppen',        type: 'merchant',    degree: 47 },
  { id: 'COOP',       label: 'Coop Sverige',        type: 'merchant',    degree: 38 },
  { id: 'CIRCLEK',    label: 'Circle K',            type: 'merchant',    degree: 35 },
  { id: 'SWEDBANK',   label: 'Swedbank',            type: 'merchant',    degree: 31 },
  { id: 'TELIA',      label: 'Telia',               type: 'merchant',    degree: 28 },
  { id: 'NORDEA',     label: 'Nordea',              type: 'merchant',    degree: 27 },
  { id: 'KLARNA',     label: 'Klarna',              type: 'merchant',    degree: 24 },
  { id: 'REVOLUT',    label: 'Revolut',             type: 'merchant',    degree: 22 },
  { id: 'SPOTIFY',    label: 'Spotify',             type: 'merchant',    degree: 19 },
  { id: 'SJ',         label: 'SJ AB',               type: 'merchant',    degree: 17 },
];

const MOCK_DUPLICATES: DuplicateCandidate[] = [
  { id: 'd1', names: ['STORACOOP', 'COOP'], confidence: 0.87, reason: 'Samma org.nr (702001-7798), delade bank-alias' },
  { id: 'd2', names: ['COMVIQ', 'TELE2'], confidence: 0.79, reason: 'Identiskt org.nr (556274-7826), varumärkesberoende' },
  { id: 'd3', names: ['HALLON', 'TRE'], confidence: 0.74, reason: 'Delar org.nr (556534-1109), varumärkesberoende' },
  { id: 'd4', names: ['ST1', 'SHELL'], confidence: 0.68, reason: 'Delade bank-alias ("SHELL"), operatörsrelation' },
  { id: 'd5', names: ['AXFOOD', 'WILLYS', 'HEMKOP'], confidence: 0.65, reason: 'Koncernrelation, delade betalningsflöden' },
];

const MOCK_COMPONENTS = [
  { id: 1, size: 23, label: 'Dagligvaror & Daglighandel' },
  { id: 2, size: 18, label: 'Bank & Fintech' },
  { id: 3, size: 15, label: 'Resor & Transport' },
  { id: 4, size: 12, label: 'Telekom & Streaming' },
  { id: 5, size: 11, label: 'Kontor & SaaS' },
  { id: 6, size: 9,  label: 'Bygg & Elektronik' },
  { id: 7, size: 8,  label: 'Drivmedel & EV-laddning' },
];

const nodeTypeColors: Record<string, string> = {
  merchant:    'bg-indigo-500/20 text-indigo-300 border-indigo-500/30',
  transaction: 'bg-green-500/20  text-green-300  border-green-500/30',
  receipt:     'bg-yellow-500/20 text-yellow-300 border-yellow-500/30',
  company:     'bg-purple-500/20 text-purple-300 border-purple-500/30',
};

function MiniGraph() {
  // Simple SVG visualization
  const nodes = [
    { x: 200, y: 120, r: 20, label: 'ICA', color: '#6366f1' },
    { x: 340, y: 80,  r: 16, label: 'COOP', color: '#6366f1' },
    { x: 360, y: 200, r: 14, label: 'CircleK', color: '#f97316' },
    { x: 100, y: 180, r: 18, label: 'Swedbank', color: '#14b8a6' },
    { x: 260, y: 230, r: 12, label: 'Telia', color: '#eab308' },
    { x: 150, y: 70,  r: 13, label: 'SJ', color: '#a855f7' },
    { x: 420, y: 130, r: 11, label: 'Klarna', color: '#14b8a6' },
    { x: 80,  y: 100, r: 10, label: 'Nordea', color: '#14b8a6' },
    { x: 310, y: 160, r: 12, label: 'Spotify', color: '#ec4899' },
  ];
  const edges = [
    [0,1],[0,3],[0,4],[1,6],[2,4],[3,7],[3,5],[4,8],[1,8],[2,6],[5,7]
  ];
  return (
    <svg viewBox="0 0 500 300" className="w-full h-48 opacity-80">
      {/* Edges */}
      {edges.map(([a,b], i) => (
        <line
          key={i}
          x1={nodes[a].x} y1={nodes[a].y}
          x2={nodes[b].x} y2={nodes[b].y}
          stroke="rgba(99,102,241,0.2)"
          strokeWidth="1.5"
        />
      ))}
      {/* Nodes */}
      {nodes.map((n, i) => (
        <g key={i} className="cursor-pointer">
          <circle
            cx={n.x} cy={n.y} r={n.r}
            fill={n.color + '33'}
            stroke={n.color}
            strokeWidth="1.5"
          />
          <text
            x={n.x} y={n.y + n.r + 10}
            textAnchor="middle"
            fill="rgba(255,255,255,0.5)"
            fontSize="8"
            fontFamily="monospace"
          >{n.label}</text>
        </g>
      ))}
    </svg>
  );
}

export default function GraphPage() {
  const [loading, setLoading] = useState(false);
  const totalNodes = MOCK_NODES.length + 96; // simulate full graph
  const totalEdges = MOCK_NODES.reduce((s, n) => s + n.degree, 0);

  return (
    <AppShell>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-white flex items-center gap-2">
            <GitGraph size={20} className="text-indigo-400" />
            Economic Graph
          </h1>
          <p className="text-xs text-white/40 mt-0.5">Relationsnätverk mellan merchants, transaktioner och underlag</p>
        </div>
        <button
          onClick={() => { setLoading(true); setTimeout(() => setLoading(false), 800); }}
          className="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-white/50 hover:text-white transition-all"
        >
          <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
        </button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <StatCard
          label="Noder"
          value={totalNodes}
          sub="merchants + transaktioner"
          accent="indigo"
          icon={<Network size={16} />}
        />
        <StatCard
          label="Edges"
          value={totalEdges}
          sub="relationer totalt"
          accent="green"
          icon={<GitMerge size={16} />}
        />
        <StatCard
          label="Komponenter"
          value={MOCK_COMPONENTS.length}
          sub="anslutna kluster"
          accent="yellow"
          icon={<TrendingUp size={16} />}
        />
      </div>

      <div className="grid grid-cols-2 gap-4 mb-4">
        {/* Graph visualization */}
        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
          <h2 className="text-sm font-medium text-white/70 mb-3 uppercase tracking-wider">Graf-preview</h2>
          <MiniGraph />
          <p className="text-[10px] text-white/30 mt-2 text-center">Visar top-9 noder efter degree. Full graf: interaktiv D3 (kommande)</p>
        </div>

        {/* Components */}
        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
          <h2 className="text-sm font-medium text-white/70 mb-3 uppercase tracking-wider">Anslutna komponenter</h2>
          <div className="space-y-2">
            {MOCK_COMPONENTS.map((comp, i) => (
              <div key={comp.id} className="flex items-center gap-3">
                <span className="text-[10px] text-white/30 w-4 text-right">{i + 1}</span>
                <div className="flex-1 h-1.5 bg-white/5 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-indigo-500/60 rounded-full"
                    style={{ width: `${(comp.size / 23) * 100}%` }}
                  />
                </div>
                <span className="text-xs text-white/50 flex-1">{comp.label}</span>
                <span className="text-xs text-white/30 font-mono">{comp.size}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* Top merchants by degree */}
        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
          <h2 className="text-sm font-medium text-white/70 mb-3 uppercase tracking-wider">Top 5 – Degree centrality</h2>
          <div className="space-y-3">
            {MOCK_NODES.slice(0, 5).map((node, i) => (
              <div key={node.id} className="flex items-center gap-3 group">
                <span className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold flex-shrink-0 ${
                  i === 0 ? 'bg-yellow-500/20 text-yellow-400' :
                  i === 1 ? 'bg-white/10 text-white/60' :
                  'bg-white/5 text-white/30'
                }`}>
                  {i + 1}
                </span>
                <div className="flex-1">
                  <div className="flex items-center justify-between mb-0.5">
                    <span className="text-sm text-white">{node.label}</span>
                    <span className="text-xs text-white/40 font-mono">{node.degree}</span>
                  </div>
                  <div className="h-1 bg-white/5 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-indigo-500/60 rounded-full transition-all duration-500"
                      style={{ width: `${(node.degree / 47) * 100}%` }}
                    />
                  </div>
                </div>
                <span className={`text-[10px] px-1.5 py-0.5 rounded border ${nodeTypeColors[node.type]}`}>
                  {node.type}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Duplicate candidates */}
        <div className="rounded-xl border border-white/5 bg-white/[0.02] p-5">
          <h2 className="text-sm font-medium text-white/70 mb-3 uppercase tracking-wider flex items-center gap-2">
            <AlertTriangle size={14} className="text-yellow-400" />
            Potentiella duplikat
          </h2>
          <div className="space-y-3">
            {MOCK_DUPLICATES.map(dup => (
              <div key={dup.id} className="group flex items-start gap-3 p-2.5 rounded-lg hover:bg-white/5 transition-all cursor-pointer">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    {dup.names.map((n, i) => (
                      <span key={n}>
                        <span className="text-xs text-white font-mono">{n}</span>
                        {i < dup.names.length - 1 && <span className="text-white/20 mx-1">≈</span>}
                      </span>
                    ))}
                  </div>
                  <p className="text-[10px] text-white/40">{dup.reason}</p>
                </div>
                <div className="flex flex-col items-end gap-1">
                  <span className={`text-[10px] font-bold ${
                    dup.confidence > 0.8 ? 'text-red-400' :
                    dup.confidence > 0.7 ? 'text-yellow-400' : 'text-white/40'
                  }`}>
                    {Math.round(dup.confidence * 100)}%
                  </span>
                  <ChevronRight size={12} className="text-white/20 group-hover:text-white/50 transition-colors" />
                </div>
              </div>
            ))}
          </div>
          <p className="text-[10px] text-white/20 mt-3 pt-3 border-t border-white/5">
            Konfidensbaserat matching via org.nr + bank-alias overlap
          </p>
        </div>
      </div>
    </AppShell>
  );
}
