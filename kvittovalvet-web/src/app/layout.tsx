import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import './globals.css';

const inter = Inter({ subsets: ['latin'] });

export const metadata: Metadata = {
  title: 'Kvittovalvet™',
  description: 'Economic Evidence Infrastructure',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="sv" className="dark">
      <body className={`${inter.className} bg-[#0a0a0f] text-white min-h-screen`}>
        {children}
      </body>
    </html>
  );
}
