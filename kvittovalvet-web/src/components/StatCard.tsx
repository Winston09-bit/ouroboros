interface Props {
  label: string;
  value: string | number;
  sub?: string;
  accent?: 'green' | 'yellow' | 'red' | 'indigo';
  icon?: React.ReactNode;
}

const accents = {
  green:  'border-green-500/20  bg-green-500/5  text-green-400',
  yellow: 'border-yellow-500/20 bg-yellow-500/5 text-yellow-400',
  red:    'border-red-500/20    bg-red-500/5    text-red-400',
  indigo: 'border-indigo-500/20 bg-indigo-500/5 text-indigo-400',
};

export default function StatCard({ label, value, sub, accent = 'indigo', icon }: Props) {
  return (
    <div className={`rounded-xl border p-4 ${accents[accent]}`}>
      <div className="flex items-start justify-between">
        <p className="text-xs font-medium opacity-70 uppercase tracking-wider">{label}</p>
        {icon && <span className="opacity-60">{icon}</span>}
      </div>
      <p className="text-2xl font-bold mt-2">{value}</p>
      {sub && <p className="text-xs opacity-50 mt-1">{sub}</p>}
    </div>
  );
}
