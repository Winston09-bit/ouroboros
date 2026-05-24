const API = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8090';

export async function apiFetch(path: string, init?: RequestInit) {
  const res = await fetch(`${API}${path}`, {
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    ...init,
  });
  if (!res.ok) throw new Error(`API ${path} → ${res.status}`);
  return res.json();
}

export const api = {
  health: ()         => apiFetch('/health'),
  transactions: ()   => apiFetch('/transactions'),
  invoices: ()       => apiFetch('/invoices'),
  providers: ()      => apiFetch('/providers'),
  roi: ()            => apiFetch('/report/roi'),
  reconcile: (body: unknown) => apiFetch('/reconcile', { method: 'POST', body: JSON.stringify(body) }),
  waitlist: (email: string)  => apiFetch('/waitlist',  { method: 'POST', body: JSON.stringify({ email }) }),
};
