'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  LayoutDashboard, ArrowLeftRight, FileText,
  ShieldCheck, Zap, Settings, ChevronRight,
  Store, GitGraph, TrendingDown,
} from 'lucide-react';

const nav = [
  { href: '/',              icon: LayoutDashboard, label: 'Dashboard'      },
  { href: '/transactions',  icon: ArrowLeftRight,  label: 'Transaktioner'  },
  { href: '/liquidity',     icon: TrendingDown,    label: 'Likviditet'      },
  { href: '/evidence',      icon: FileText,        label: 'Underlag'        },
  { href: '/audit',         icon: ShieldCheck,     label: 'Revision'        },
  { href: '/escalations',   icon: Zap,             label: 'Eskaleringar'   },
  { href: '/merchants',     icon: Store,           label: 'Merchants'       },
  { href: '/graph',         icon: GitGraph,        label: 'Graph'           },
  { href: '/settings',      icon: Settings,        label: 'Inställningar'   },
];

export default function Sidebar() {
  const path = usePathname();
  return (
    <aside className="w-56 flex-shrink-0 border-r border-white/5 bg-[#0d0d14] flex flex-col">
      {/* Logo */}
      <div className="px-5 py-5 border-b border-white/5">
        <p className="text-xs font-semibold tracking-widest text-indigo-400 uppercase">Kvittovalvet™</p>
        <p className="text-[10px] text-white/30 mt-0.5">Economic Evidence Infrastructure</p>
      </div>

      {/* Nav */}
      <nav className="flex-1 py-4 px-2 space-y-0.5">
        {nav.map(({ href, icon: Icon, label }) => {
          const active = path === href;
          return (
            <Link
              key={href}
              href={href}
              className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all group ${
                active
                  ? 'bg-indigo-600/20 text-indigo-300 font-medium'
                  : 'text-white/50 hover:text-white hover:bg-white/5'
              }`}
            >
              <Icon size={15} className={active ? 'text-indigo-400' : ''} />
              <span className="flex-1">{label}</span>
              {active && <ChevronRight size={12} className="text-indigo-400" />}
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="px-5 py-4 border-t border-white/5">
        <p className="text-[10px] text-white/20">LandveX AB · v2.0.0</p>
      </div>
    </aside>
  );
}
