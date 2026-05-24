'use client';
import { useState, useMemo } from 'react';
import AppShell from '@/components/AppShell';
import {
  Search, X, ExternalLink, ChevronRight,
  Store, Zap, Globe, Mail, Smartphone,
  CheckCircle2, XCircle, ChevronDown,
} from 'lucide-react';

interface Merchant {
  merchant_id: string;
  display_name: string;
  category: string;
  org_number: string | null;
  has_api_access: boolean;
  receipt_support_channels: string[];
  bank_aliases: string[];
  website: string;
  notes: string;
}

const MERCHANTS: Merchant[] = [
  { merchant_id: "ICA", display_name: "ICA Gruppen", category: "DAGLIGVAROR", org_number: "556015-0875", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["ICA","ICA MAXI","ICA KVANTUM","ICA SUPERMARKET","ICA NARA","ICA NÄRA"], website: "https://www.ica.se", notes: "Digitala kvitton via ICA-appen om Stamkundskort/ICA-kort används vid köp." },
  { merchant_id: "COOP", display_name: "Coop Sverige", category: "DAGLIGVAROR", org_number: "702001-7798", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["COOP","COOP FORUM","COOP EXTRA","COOP NARA"], website: "https://www.coop.se", notes: "Digitalt kvitto via Coop-appen med Coop-kort." },
  { merchant_id: "WILLYS", display_name: "Willys", category: "DAGLIGVAROR", org_number: "556544-6244", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["WILLYS","WILLYS HEMMA","WILLYS PLUS","WILLYS CITY"], website: "https://www.willys.se", notes: "Del av Axfood-koncernen. Digitalt kvitto via Willys-appen (Triss)." },
  { merchant_id: "HEMKOP", display_name: "Hemköp", category: "DAGLIGVAROR", org_number: "556018-3215", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["HEMKOP","HEM KÖP","HEMKÖP"], website: "https://www.hemkop.se", notes: "Del av Axfood. Digitalt kvitto via Hemköp-appen." },
  { merchant_id: "CITYGROSS", display_name: "City Gross", category: "DAGLIGVAROR", org_number: "556021-9415", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["CITY GROSS","CITYGROSS"], website: "https://www.citygross.se", notes: "Fokus på södra Sverige." },
  { merchant_id: "LIDL", display_name: "Lidl Sverige", category: "DAGLIGVAROR", org_number: "556453-3011", has_api_access: false, receipt_support_channels: ["app","email"], bank_aliases: ["LIDL","LIDL SVERIGE","LIDL STOCKHOLM"], website: "https://www.lidl.se", notes: "Lidl Plus-appen ger digitala kvitton." },
  { merchant_id: "STORACOOP", display_name: "Stora Coop", category: "DAGLIGVAROR", org_number: "702001-7798", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["STORA COOP","STORACOOP"], website: "https://www.coop.se/stora-coop/", notes: "Samma system som Coop." },
  { merchant_id: "AXFOOD", display_name: "Axfood", category: "DAGLIGVAROR", org_number: "556542-0824", has_api_access: false, receipt_support_channels: ["web","email"], bank_aliases: ["AXFOOD","AXFOOD AB"], website: "https://www.axfood.se", notes: "Moderbolag till Willys, Hemköp, Tempo, Snabbgross." },
  { merchant_id: "CIRCLEK", display_name: "Circle K Sverige", category: "DRIVMEDEL", org_number: "556008-5674", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["CIRCLE K","CIRCLEK","STATOIL"], website: "https://www.circlek.se", notes: "Tidigare Statoil. Circle K Extra-appen ger digitala kvitton." },
  { merchant_id: "OKQ8", display_name: "OKQ8", category: "DRIVMEDEL", org_number: "556037-2859", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["OKQ8","OK Q8","Q8"], website: "https://www.okq8.se", notes: "OKQ8 Duo-kort ger kvittohistorik online." },
  { merchant_id: "ST1", display_name: "ST1", category: "DRIVMEDEL", org_number: "556669-5553", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["ST1","SHELL EXPRESS","SHELL"], website: "https://www.st1.se", notes: "ST1 driver även Shell i Sverige." },
  { merchant_id: "PREEM", display_name: "Preem", category: "DRIVMEDEL", org_number: "556072-6685", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["PREEM","PREEM BENSIN"], website: "https://www.preem.se", notes: "Preem Mastercard ger full kvittohistorik." },
  { merchant_id: "TESLA_SUPERCHARGING", display_name: "Tesla Supercharging", category: "DRIVMEDEL", org_number: null, has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["TESLA","TESLA SUPERCHARGING"], website: "https://www.tesla.com/sv_SE/charging", notes: "Kvitto automatiskt skickat till Tesla-kontots e-post efter varje laddning." },
  { merchant_id: "CHARGENODE", display_name: "ChargeNode", category: "DRIVMEDEL", org_number: "559038-6042", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["CHARGENODE","CHARGE NODE"], website: "https://www.chargenode.com", notes: "Nordisk laddnätverk. API tillgängligt." },
  { merchant_id: "VATTENFALL_INCHARGE", display_name: "Vattenfall InCharge", category: "DRIVMEDEL", org_number: "556036-2316", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["VATTENFALL","VATTENFALL INCHARGE","INCHARGE"], website: "https://www.vattenfall.se/incharge/", notes: "Vattenfalls laddnätverk för elbilar." },
  { merchant_id: "MER_CHARGING", display_name: "Mer (laddnätverk)", category: "DRIVMEDEL", org_number: "559024-1832", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["MER","MER CHARGING"], website: "https://www.mer.eco", notes: "Nordiskt laddnätverk (f.d. Grønn Kontakt/Recharge)." },
  { merchant_id: "CLASOHLSON", display_name: "Clas Ohlson", category: "BYGG_ELEKTRONIK", org_number: "556035-8231", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["CLAS OHLSON","CLASOHLSON"], website: "https://www.clasohlson.com/se/", notes: "Clas Ohlson Club-kortet ger digital kvittohistorik." },
  { merchant_id: "BAUHAUS", display_name: "Bauhaus Sverige", category: "BYGG_ELEKTRONIK", org_number: "556212-9234", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["BAUHAUS","BAUHAUS BYGG"], website: "https://www.bauhaus.se", notes: "Bauhaus Club-kort ger kvittokopia online." },
  { merchant_id: "BILTEMA", display_name: "Biltema", category: "BYGG_ELEKTRONIK", org_number: "556024-1006", has_api_access: false, receipt_support_channels: ["web","email"], bank_aliases: ["BILTEMA","BILTEMA AB"], website: "https://www.biltema.se", notes: "Biltema-konto ger kvittohistorik online." },
  { merchant_id: "JULA", display_name: "Jula", category: "BYGG_ELEKTRONIK", org_number: "556045-9573", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["JULA","JULA AB"], website: "https://www.jula.se", notes: "Jula Club-kort ger digital kvittohistorik i Jula-appen." },
  { merchant_id: "ELGIGANTEN", display_name: "Elgiganten", category: "BYGG_ELEKTRONIK", org_number: "556286-3449", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["ELGIGANTEN","ELGIGANTEN AB"], website: "https://www.elgiganten.se", notes: "Del av Elkjøp Nordic. Kvitto i appen eller via e-post." },
  { merchant_id: "MEDIAMARKT", display_name: "MediaMarkt", category: "BYGG_ELEKTRONIK", org_number: "556421-6693", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["MEDIAMARKT","MEDIA MARKT"], website: "https://www.mediamarkt.se", notes: "Tysk elektronikkedja. MediaMarkt-appen ger digital kvittohistorik." },
  { merchant_id: "KJELL", display_name: "Kjell & Company", category: "BYGG_ELEKTRONIK", org_number: "556400-5773", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["KJELL","KJELL & CO"], website: "https://www.kjell.com/se/", notes: "Kvitto i Kjell & Company-appen. 365 dagars öppet köp." },
  { merchant_id: "SCANDIC", display_name: "Scandic Hotels", category: "HOTELL_RESOR", org_number: "556703-1702", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["SCANDIC","SCANDIC HOTELS"], website: "https://www.scandichotels.se", notes: "Kvitto (folio) skickas automatiskt till e-post vid utcheckning." },
  { merchant_id: "SJ", display_name: "SJ AB", category: "HOTELL_RESOR", org_number: "556388-3077", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["SJ","SJ AB","SJ TÅGET"], website: "https://www.sj.se", notes: "Digitala biljetter och kvitton i SJ-appen." },
  { merchant_id: "SAS", display_name: "SAS (Scandinavian Airlines)", category: "HOTELL_RESOR", org_number: "556102-4223", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["SAS","SCANDINAVIAN AIRLINES"], website: "https://www.sas.se", notes: "E-postkvitto automatiskt. EuroBonus-app." },
  { merchant_id: "NORWEGIAN", display_name: "Norwegian Air Shuttle", category: "HOTELL_RESOR", org_number: null, has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["NORWEGIAN","NORWEGIAN AIR"], website: "https://www.norwegian.com/se/", notes: "Norskt lågprisflygbolag. E-postkvitto automatiskt." },
  { merchant_id: "BOOKINGCOM", display_name: "Booking.com", category: "HOTELL_RESOR", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["BOOKING.COM","BOOKING"], website: "https://www.booking.com", notes: "Global hotellbokningsplattform. Kvitto/faktura via Mina bokningar." },
  { merchant_id: "AIRBNB", display_name: "Airbnb", category: "HOTELL_RESOR", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["AIRBNB","AIR BNB"], website: "https://www.airbnb.se", notes: "Korttidsbokning. Automatiskt e-postkvitto." },
  { merchant_id: "MCDONALDS", display_name: "McDonald's Sverige", category: "RESTAURANG", org_number: "556015-3541", has_api_access: false, receipt_support_channels: ["app"], bank_aliases: ["MCDONALDS","MC DONALDS"], website: "https://www.mcdonalds.com/se/sv-se.html", notes: "McDonald's-appen ger digitala kvitton." },
  { merchant_id: "MAXBURGERS", display_name: "Max Burgers", category: "RESTAURANG", org_number: "556020-4219", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["MAX","MAX BURGERS"], website: "https://www.max.se", notes: "Svensk hamburgerkedja. Max-appen (MAX Plus) ger digitala kvitton." },
  { merchant_id: "ESPRESSOHOUSE", display_name: "Espresso House", category: "RESTAURANG", org_number: "556583-7428", has_api_access: false, receipt_support_channels: ["app"], bank_aliases: ["ESPRESSO HOUSE","ESPRESSOHOUSE"], website: "https://www.espressohouse.com/se/", notes: "Nordisk kaffekedja. Stars-programmet: digitala kvitton." },
  { merchant_id: "BOLT", display_name: "Bolt", category: "TRANSPORT", org_number: null, has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["BOLT","BOLT EU"], website: "https://bolt.eu/sv-se/", notes: "Estländsk ride-hailing. Automatiskt e-postkvitto efter varje resa." },
  { merchant_id: "UBER", display_name: "Uber", category: "TRANSPORT", org_number: null, has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["UBER","UBER BV","UBER TAXI"], website: "https://www.uber.com/se/sv/", notes: "Globalt ride-hailing. Automatiskt e-postkvitto." },
  { merchant_id: "SL", display_name: "SL (Storstockholms Lokaltrafik)", category: "TRANSPORT", org_number: "556013-0683", has_api_access: true, receipt_support_channels: ["app","web"], bank_aliases: ["SL","SL AB"], website: "https://sl.se", notes: "Kollektivtrafik i Storstockholmsregionen." },
  { merchant_id: "TELIA", display_name: "Telia", category: "TELEKOM", org_number: "556103-4249", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["TELIA","TELIA SE"], website: "https://www.telia.se", notes: "Telekommunikation, mobil, bredband." },
  { merchant_id: "TELE2", display_name: "Tele2", category: "TELEKOM", org_number: "556274-7826", has_api_access: false, receipt_support_channels: ["web","email"], bank_aliases: ["TELE2","TELE 2"], website: "https://www.tele2.se", notes: "Mobil och bredband." },
  { merchant_id: "TELENOR", display_name: "Telenor Sverige", category: "TELEKOM", org_number: "556421-0250", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["TELENOR","TELENOR SE"], website: "https://www.telenor.se", notes: "Norskt telekombolag med stor svensk närvaro." },
  { merchant_id: "NETFLIX", display_name: "Netflix", category: "STREAMING", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["NETFLIX","NETFLIX INC"], website: "https://www.netflix.com/se/", notes: "Strömning. Månadsvis kvitto/faktura skickas till e-post." },
  { merchant_id: "SPOTIFY", display_name: "Spotify", category: "STREAMING", org_number: "556703-7485", has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["SPOTIFY","SPOTIFY AB"], website: "https://www.spotify.com/se/", notes: "Svensk musikströmning. Kvitto skickas till e-post månadsvis." },
  { merchant_id: "DISNEYPLUS", display_name: "Disney+", category: "STREAMING", org_number: null, has_api_access: false, receipt_support_channels: ["web","email"], bank_aliases: ["DISNEY+","DISNEY PLUS"], website: "https://www.disneyplus.com/sv-se", notes: "Disneys strömtjänst. E-postkvitto månadsvis." },
  { merchant_id: "VIAPLAY", display_name: "Viaplay", category: "STREAMING", org_number: "556442-7564", has_api_access: false, receipt_support_channels: ["web","email"], bank_aliases: ["VIAPLAY","VIASAT"], website: "https://viaplay.se", notes: "Nordisk strömtjänst." },
  { merchant_id: "ADOBE", display_name: "Adobe", category: "KONTOR", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["ADOBE","ADOBE INC","ADOBE*"], website: "https://www.adobe.com/se/", notes: "Creative Cloud-prenumeration. Månadskvitto till e-post." },
  { merchant_id: "MICROSOFT", display_name: "Microsoft", category: "KONTOR", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["MICROSOFT","MICROSOFT*","MSFT"], website: "https://www.microsoft.com/sv-se/", notes: "Microsoft 365, Azure. Kvitto till e-post och i Microsoft-kontoportalen." },
  { merchant_id: "GOOGLE_WORKSPACE", display_name: "Google Workspace", category: "KONTOR", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["GOOGLE","GOOGLE WORKSPACE"], website: "https://workspace.google.com/", notes: "Google Workspace (GMail, Drive). Faktura/kvitto via Google Admin-konsolen." },
  { merchant_id: "AWS", display_name: "Amazon Web Services", category: "KONTOR", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["AWS","AMAZON WEB SERVICES"], website: "https://aws.amazon.com/", notes: "Cloud-infrastruktur. Månadsvis faktura via AWS Billing Console." },
  { merchant_id: "GITHUB", display_name: "GitHub", category: "KONTOR", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["GITHUB","GITHUB INC"], website: "https://github.com", notes: "Versionskontroll och CI/CD. Kvitto till e-post månadsvis." },
  { merchant_id: "NOTION", display_name: "Notion", category: "KONTOR", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["NOTION","NOTION LABS"], website: "https://www.notion.so", notes: "Workspace/anteckningsverktyg. Kvitto till e-post månadsvis/årsvis." },
  { merchant_id: "IKEA", display_name: "IKEA", category: "KONTOR", org_number: "556084-9806", has_api_access: false, receipt_support_channels: ["app","web","email"], bank_aliases: ["IKEA","IKEA AB"], website: "https://www.ikea.com/se/sv/", notes: "IKEA Family-kort: kvittohistorik." },
  { merchant_id: "SWEDBANK", display_name: "Swedbank", category: "BANK_FINANS", org_number: "502017-7753", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["SWEDBANK","SWEDBANK AB"], website: "https://www.swedbank.se", notes: "Storbank. Open Banking API." },
  { merchant_id: "NORDEA", display_name: "Nordea", category: "BANK_FINANS", org_number: "516406-0120", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["NORDEA","NORDEA BANK"], website: "https://www.nordea.se", notes: "Nordisk storbank. Open Banking API." },
  { merchant_id: "REVOLUT", display_name: "Revolut", category: "BANK_FINANS", org_number: null, has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["REVOLUT","REVOLUT LTD"], website: "https://www.revolut.com/sv-SE/", notes: "Neobank. Full transaktionshistorik och kvitton i appen." },
  { merchant_id: "KLARNA", display_name: "Klarna", category: "BANK_FINANS", org_number: "556737-0431", has_api_access: true, receipt_support_channels: ["app","web","email"], bank_aliases: ["KLARNA","KLARNA AB"], website: "https://www.klarna.com/se/", notes: "Betaltjänst och neobank. Full köphistorik i Klarna-appen." },
  { merchant_id: "STRIPE", display_name: "Stripe", category: "BANK_FINANS", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["STRIPE","STRIPE INC"], website: "https://stripe.com/se", notes: "Betalningsinfrastruktur för utvecklare. Kvitto/faktura via Stripe Dashboard." },
  { merchant_id: "TINK", display_name: "Tink", category: "BANK_FINANS", org_number: "556898-8887", has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["TINK","TINK AB"], website: "https://tink.com/se/", notes: "Open Banking-plattform (ägt av Visa). Komplett API." },
  { merchant_id: "WISE", display_name: "Wise (TransferWise)", category: "BANK_FINANS", org_number: null, has_api_access: true, receipt_support_channels: ["web","email"], bank_aliases: ["WISE","TRANSFERWISE"], website: "https://wise.com/se/", notes: "Internationella överföringar och multi-valutakonto." },
];

const CATEGORY_COLORS: Record<string, string> = {
  DAGLIGVAROR:   'bg-green-500/15  text-green-400  border-green-500/20',
  DRIVMEDEL:     'bg-orange-500/15 text-orange-400 border-orange-500/20',
  BYGG_ELEKTRONIK:'bg-blue-500/15  text-blue-400   border-blue-500/20',
  HOTELL_RESOR:  'bg-purple-500/15 text-purple-400 border-purple-500/20',
  RESTAURANG:    'bg-red-500/15    text-red-400    border-red-500/20',
  TRANSPORT:     'bg-cyan-500/15   text-cyan-400   border-cyan-500/20',
  TELEKOM:       'bg-yellow-500/15 text-yellow-400 border-yellow-500/20',
  STREAMING:     'bg-pink-500/15   text-pink-400   border-pink-500/20',
  KONTOR:        'bg-indigo-500/15 text-indigo-400 border-indigo-500/20',
  BANK_FINANS:   'bg-teal-500/15   text-teal-400   border-teal-500/20',
};

const CATEGORIES = Array.from(new Set(MERCHANTS.map(m => m.category)));

const channelIcon = (ch: string) => {
  if (ch === 'app')   return <Smartphone size={11} key={ch} className="text-white/50" />;
  if (ch === 'web')   return <Globe size={11} key={ch} className="text-white/50" />;
  if (ch === 'email') return <Mail size={11} key={ch} className="text-white/50" />;
  return null;
};

const categoryLabel = (cat: string) => cat.replace(/_/g, ' ');

export default function MerchantsPage() {
  const [search, setSearch]         = useState('');
  const [catFilter, setCatFilter]   = useState<string>('');
  const [apiFilter, setApiFilter]   = useState<boolean | null>(null);
  const [selected, setSelected]     = useState<Merchant | null>(null);
  const [showCatMenu, setShowCatMenu] = useState(false);

  const filtered = useMemo(() => {
    return MERCHANTS.filter(m => {
      const q = search.toLowerCase();
      const matchSearch = !q || m.display_name.toLowerCase().includes(q)
        || m.merchant_id.toLowerCase().includes(q)
        || m.bank_aliases.some(a => a.toLowerCase().includes(q));
      const matchCat = !catFilter || m.category === catFilter;
      const matchApi = apiFilter === null || m.has_api_access === apiFilter;
      return matchSearch && matchCat && matchApi;
    });
  }, [search, catFilter, apiFilter]);

  const catCounts = useMemo(() => {
    const c: Record<string, number> = {};
    MERCHANTS.forEach(m => { c[m.category] = (c[m.category] || 0) + 1; });
    return c;
  }, []);

  return (
    <AppShell>
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-white flex items-center gap-2">
            <Store size={20} className="text-indigo-400" />
            Merchants
          </h1>
          <p className="text-xs text-white/40 mt-0.5">
            {MERCHANTS.length} merchants · {Object.keys(catCounts).length} kategorier
          </p>
        </div>
        {/* Category pills */}
        <div className="flex flex-wrap gap-1.5 max-w-lg justify-end">
          {CATEGORIES.map(cat => (
            <button
              key={cat}
              onClick={() => setCatFilter(prev => prev === cat ? '' : cat)}
              className={`text-[10px] px-2 py-0.5 rounded-full border font-medium transition-all ${
                catFilter === cat
                  ? CATEGORY_COLORS[cat]
                  : 'border-white/10 text-white/30 hover:text-white/60 hover:border-white/20'
              }`}
            >
              {categoryLabel(cat)} {catCounts[cat]}
            </button>
          ))}
        </div>
      </div>

      {/* Filters bar */}
      <div className="flex items-center gap-3 mb-4">
        <div className="relative flex-1 max-w-xs">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-white/30" />
          <input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Sök merchant, alias…"
            className="w-full bg-white/5 border border-white/10 rounded-lg pl-9 pr-4 py-2 text-sm text-white placeholder-white/30 focus:outline-none focus:border-indigo-500/50"
          />
          {search && (
            <button onClick={() => setSearch('')} className="absolute right-3 top-1/2 -translate-y-1/2 text-white/30 hover:text-white">
              <X size={12} />
            </button>
          )}
        </div>

        {/* API filter */}
        <div className="flex items-center gap-1 bg-white/5 border border-white/10 rounded-lg p-1">
          {[null, true, false].map(v => (
            <button
              key={String(v)}
              onClick={() => setApiFilter(v)}
              className={`px-3 py-1 rounded text-xs transition-all ${
                apiFilter === v
                  ? 'bg-indigo-600 text-white'
                  : 'text-white/40 hover:text-white'
              }`}
            >
              {v === null ? 'Alla' : v ? 'Har API' : 'Utan API'}
            </button>
          ))}
        </div>

        <span className="text-xs text-white/30 ml-auto">{filtered.length} resultat</span>
      </div>

      {/* Table */}
      <div className="rounded-xl border border-white/5 overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-white/5 bg-white/[0.02]">
              <th className="px-4 py-3 text-left text-xs font-medium text-white/40 uppercase tracking-wider">Merchant</th>
              <th className="px-4 py-3 text-left text-xs font-medium text-white/40 uppercase tracking-wider">Kategori</th>
              <th className="px-4 py-3 text-left text-xs font-medium text-white/40 uppercase tracking-wider">Kvittostöd</th>
              <th className="px-4 py-3 text-left text-xs font-medium text-white/40 uppercase tracking-wider">API</th>
              <th className="px-4 py-3 text-left text-xs font-medium text-white/40 uppercase tracking-wider">Org.nr</th>
              <th className="px-4 py-3 text-right text-xs font-medium text-white/40 uppercase tracking-wider"></th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((m, i) => (
              <tr
                key={m.merchant_id}
                onClick={() => setSelected(m)}
                className={`border-b border-white/[0.03] cursor-pointer transition-all hover:bg-indigo-500/5 ${
                  i % 2 === 0 ? '' : 'bg-white/[0.01]'
                } ${selected?.merchant_id === m.merchant_id ? 'bg-indigo-500/10' : ''}`}
              >
                <td className="px-4 py-3">
                  <div className="flex items-center gap-3">
                    <div className="w-7 h-7 rounded-lg bg-white/5 flex items-center justify-center text-[10px] font-bold text-white/50 flex-shrink-0">
                      {m.display_name.slice(0, 2).toUpperCase()}
                    </div>
                    <div>
                      <p className="text-white font-medium">{m.display_name}</p>
                      <p className="text-[10px] text-white/30">{m.merchant_id}</p>
                    </div>
                  </div>
                </td>
                <td className="px-4 py-3">
                  <span className={`text-[10px] px-2 py-0.5 rounded-full border font-medium ${CATEGORY_COLORS[m.category] || 'bg-white/5 text-white/40 border-white/10'}`}>
                    {categoryLabel(m.category)}
                  </span>
                </td>
                <td className="px-4 py-3">
                  <div className="flex items-center gap-1.5">
                    {m.receipt_support_channels.map(channelIcon)}
                    <span className="text-[10px] text-white/30">{m.receipt_support_channels.join(', ')}</span>
                  </div>
                </td>
                <td className="px-4 py-3">
                  {m.has_api_access
                    ? <span className="flex items-center gap-1 text-green-400 text-xs"><CheckCircle2 size={12} /> Ja</span>
                    : <span className="flex items-center gap-1 text-white/30 text-xs"><XCircle size={12} /> Nej</span>
                  }
                </td>
                <td className="px-4 py-3 text-xs text-white/40 font-mono">
                  {m.org_number || '–'}
                </td>
                <td className="px-4 py-3 text-right">
                  <ChevronRight size={14} className="text-white/20 inline" />
                </td>
              </tr>
            ))}
            {filtered.length === 0 && (
              <tr>
                <td colSpan={6} className="px-4 py-12 text-center text-white/30 text-sm">
                  Inga merchants matchar sökningen
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Drawer */}
      {selected && (
        <div className="fixed inset-0 z-50 flex" onClick={() => setSelected(null)}>
          <div className="flex-1 bg-black/50 backdrop-blur-sm" />
          <div
            className="w-96 bg-[#0d0d14] border-l border-white/5 h-full overflow-y-auto p-6 flex flex-col gap-5"
            onClick={e => e.stopPropagation()}
          >
            {/* Drawer header */}
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-indigo-600/20 flex items-center justify-center text-sm font-bold text-indigo-300">
                  {selected.display_name.slice(0, 2).toUpperCase()}
                </div>
                <div>
                  <h2 className="text-base font-semibold text-white">{selected.display_name}</h2>
                  <p className="text-xs text-white/40">{selected.merchant_id}</p>
                </div>
              </div>
              <button onClick={() => setSelected(null)} className="text-white/30 hover:text-white p-1">
                <X size={16} />
              </button>
            </div>

            {/* Category */}
            <div>
              <p className="text-[10px] text-white/30 uppercase tracking-wider mb-1.5">Kategori</p>
              <span className={`text-xs px-2.5 py-1 rounded-full border font-medium ${CATEGORY_COLORS[selected.category]}`}>
                {categoryLabel(selected.category)}
              </span>
            </div>

            {/* API access */}
            <div className="flex items-center gap-3 px-3 py-2.5 rounded-lg bg-white/5">
              {selected.has_api_access
                ? <><CheckCircle2 size={16} className="text-green-400" /><span className="text-sm text-green-400 font-medium">API-åtkomst tillgänglig</span></>
                : <><XCircle size={16} className="text-white/40" /><span className="text-sm text-white/40">Ingen API-åtkomst</span></>
              }
            </div>

            {/* Kvittostöd */}
            <div>
              <p className="text-[10px] text-white/30 uppercase tracking-wider mb-2">Kvittokanaler</p>
              <div className="flex flex-wrap gap-2">
                {selected.receipt_support_channels.map(ch => (
                  <span key={ch} className="flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-lg bg-white/5 text-white/60">
                    {channelIcon(ch)}
                    {ch}
                  </span>
                ))}
              </div>
            </div>

            {/* Bank aliases */}
            <div>
              <p className="text-[10px] text-white/30 uppercase tracking-wider mb-2">Bank-alias ({selected.bank_aliases.length})</p>
              <div className="flex flex-wrap gap-1.5">
                {selected.bank_aliases.map(alias => (
                  <span key={alias} className="text-[10px] px-2 py-0.5 rounded bg-white/5 text-white/50 font-mono border border-white/5">
                    {alias}
                  </span>
                ))}
              </div>
            </div>

            {/* Org nr */}
            {selected.org_number && (
              <div>
                <p className="text-[10px] text-white/30 uppercase tracking-wider mb-1.5">Org.nr</p>
                <p className="text-sm text-white font-mono">{selected.org_number}</p>
              </div>
            )}

            {/* Notes */}
            {selected.notes && (
              <div>
                <p className="text-[10px] text-white/30 uppercase tracking-wider mb-1.5">Anteckningar</p>
                <p className="text-xs text-white/50 leading-relaxed">{selected.notes}</p>
              </div>
            )}

            {/* Website */}
            {selected.website && (
              <a
                href={selected.website}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-xs text-indigo-400 hover:text-indigo-300 transition-colors"
              >
                <ExternalLink size={12} />
                {selected.website}
              </a>
            )}
          </div>
        </div>
      )}
    </AppShell>
  );
}
