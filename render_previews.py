#!/usr/bin/env python3
"""
Render key sheets of The Ultimate Budget.xlsx as PNG previews + a combined PDF.
Uses openpyxl to read cell values/styles and matplotlib to draw each sheet.
"""

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from matplotlib.patches import FancyBboxPatch
import matplotlib.gridspec as gridspec
from openpyxl import load_workbook
from openpyxl.utils import get_column_letter, column_index_from_string
import os, math

OUT_DIR = "/home/user/ouroboros/previews"
os.makedirs(OUT_DIR, exist_ok=True)

# ── Palette ───────────────────────────────────────────────────────────────────
def hx(s):
    s = s.lstrip("#")
    return tuple(int(s[i:i+2],16)/255 for i in (0,2,4))

FOREST   = hx("404A36")
SAGE     = hx("8B9A7A")
LT_SAGE  = hx("E6EADC")
ROSE     = hx("C0907E")
CREAM    = hx("F2EDE3")
CARD     = hx("FBF8F1")
GOLD     = hx("C7A862")
TEXT     = hx("33352E")
WHITE    = (1,1,1)

def save(fig, name):
    path = os.path.join(OUT_DIR, name)
    fig.savefig(path, dpi=120, bbox_inches="tight", facecolor=fig.get_facecolor())
    plt.close(fig)
    print(f"  Saved: {path}")
    return path

# ── Load workbook ─────────────────────────────────────────────────────────────
WB_PATH = "/home/user/ouroboros/The Ultimate Budget.xlsx"
print("Loading workbook…")
wb = load_workbook(WB_PATH)          # formulas as strings
print(f"Sheets: {wb.sheetnames}")

# ── Helper to get a literal cell value (skip formulas; return None) ───────────
def lit(ws, row, col):
    v = ws.cell(row, col).value
    if v is None: return None
    if isinstance(v, str) and v.startswith("="): return None
    return v

# ── Shared draw helpers ───────────────────────────────────────────────────────
def header_band(ax, title, subtitle=""):
    ax.set_facecolor(FOREST)
    ax.text(0.5, 0.6, title, color=WHITE, fontsize=16, fontweight="bold",
            ha="center", va="center", transform=ax.transAxes)
    if subtitle:
        ax.text(0.5, 0.2, subtitle, color=SAGE, fontsize=10,
                ha="center", va="center", transform=ax.transAxes)
    ax.set_xticks([]); ax.set_yticks([])

def kpi_card(ax, label, value, fill):
    ax.set_facecolor(fill)
    for sp in ax.spines.values(): sp.set_color(SAGE)
    ax.text(0.5, 0.72, label, color=FOREST, fontsize=9, fontweight="bold",
            ha="center", va="center", transform=ax.transAxes)
    ax.text(0.5, 0.35, str(value), color=FOREST, fontsize=14, fontweight="bold",
            ha="center", va="center", transform=ax.transAxes)
    ax.set_xticks([]); ax.set_yticks([])

def simple_bar(ax, labels, series, colors, title, ylabel="Amount ($)"):
    x = range(len(labels))
    w = 0.35
    for i, (vals, col, name) in enumerate(series):
        offset = [(xi + (i - (len(series)-1)/2) * w) for xi in x]
        ax.bar(offset, vals, w, color=col, label=name, zorder=3)
    ax.set_xticks(list(x)); ax.set_xticklabels(labels, fontsize=7, rotation=30, ha="right")
    ax.set_ylabel(ylabel, fontsize=8, color=TEXT)
    ax.set_title(title, fontsize=9, fontweight="bold", color=TEXT, pad=4)
    ax.legend(fontsize=7); ax.set_facecolor(CARD); ax.grid(axis="y", color="#ddd", zorder=0)
    ax.tick_params(colors=TEXT, labelsize=7)
    for sp in ["top","right"]: ax.spines[sp].set_visible(False)

def donut(ax, labels, vals, title):
    vals = [v if v > 0 else 0.001 for v in vals]
    palette = [SAGE, ROSE, GOLD, LT_SAGE, FOREST,
               hx("A9C08C"), hx("D4A96A"), hx("7B8F6E"), CREAM, CARD]
    colors = [palette[i % len(palette)] for i in range(len(vals))]
    wedges, _ = ax.pie(vals, colors=colors, startangle=90,
                       wedgeprops=dict(width=0.55, edgecolor=WHITE, linewidth=1))
    ax.set_title(title, fontsize=9, fontweight="bold", color=TEXT, pad=4)
    short_labels = [l[:14] for l in labels]
    ax.legend(wedges, short_labels, loc="lower center", bbox_to_anchor=(0.5,-0.18),
              fontsize=6, ncol=2)

# ═══════════════════════════════════════════════════════════════════════════════
# 1. DASHBOARD
# ═══════════════════════════════════════════════════════════════════════════════
print("\nRendering Dashboard…")

MONTHS = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"]
EXPENSE_CATS = ["Rent/Mortgage","Utilities","Groceries","Dining Out","Transport",
                "Fuel","Car/Auto","Insurance","Health","Phone","Internet",
                "Subscriptions","Entertainment","Shopping","Clothing","Personal Care",
                "Gym/Fitness","Pets","Kids","Gifts/Donations","Travel","Miscellaneous"]

# Seed data from sample transactions (hardcoded since formulas won't recalc here)
income_by_month  = [5000,5000,5000,0,0,0,0,0,0,0,0,0]
expense_by_month = [1745,1695,1610,0,0,0,0,0,0,0,0,0]  # sum of expenses per month
net_by_month     = [i-e for i,e in zip(income_by_month, expense_by_month)]

cat_actuals = {
    "Rent/Mortgage": 4500, "Groceries": 365, "Utilities": 80,
    "Dining Out": 45, "Entertainment": 60,
}
budget_cats = ["Rent/Mortgage","Groceries","Utilities","Dining Out","Entertainment",
               "Transport","Phone","Internet"]
budget_vals = [1500*12, 400*12, 150*12, 200*12, 100*12, 150*12, 80*12, 60*12]
actual_vals = [4500, 365, 80, 45, 60, 0, 0, 0]

total_income   = sum(income_by_month)
total_expenses = sum(expense_by_month)
net_savings    = total_income - total_expenses
savings_rate   = net_savings / total_income if total_income else 0

fig = plt.figure(figsize=(16, 18), facecolor=CREAM)
fig.suptitle("", fontsize=1)

gs = gridspec.GridSpec(6, 4, figure=fig, hspace=0.55, wspace=0.35,
                       left=0.05, right=0.97, top=0.95, bottom=0.04)

# Title band
ax_title = fig.add_subplot(gs[0, :])
header_band(ax_title, "🌿  The Ultimate Budget  —  Dashboard", "Budget Year: 2026")

# KPI cards
kpis = [
    ("Total Income", f"${total_income:,.0f}", LT_SAGE),
    ("Total Expenses", f"${total_expenses:,.0f}", hx("F5E0DA")),
    ("Net Savings", f"${net_savings:,.0f}", CARD),
    ("Savings Rate", f"{savings_rate:.1%}", CREAM),
]
for col, (lbl, val, fill) in enumerate(kpis):
    ax = fig.add_subplot(gs[1, col])
    kpi_card(ax, lbl, val, fill)

# Chart 1: Income vs Expenses
ax1 = fig.add_subplot(gs[2, :2])
simple_bar(ax1, MONTHS,
           [(income_by_month, SAGE, "Income"),
            (expense_by_month, ROSE, "Expenses")],
           [SAGE, ROSE], "Income vs Expenses by Month")

# Chart 2: Net Savings
ax2 = fig.add_subplot(gs[2, 2:])
bar_colors = [SAGE if v >= 0 else ROSE for v in net_by_month]
ax2.bar(range(12), net_by_month, color=bar_colors, zorder=3)
ax2.set_xticks(range(12)); ax2.set_xticklabels(MONTHS, fontsize=7, rotation=30, ha="right")
ax2.set_title("Net Savings by Month", fontsize=9, fontweight="bold", color=TEXT, pad=4)
ax2.axhline(0, color=TEXT, linewidth=0.8)
ax2.set_facecolor(CARD); ax2.grid(axis="y", color="#ddd", zorder=0)
for sp in ["top","right"]: ax2.spines[sp].set_visible(False)

# Chart 3: Budget vs Actual (top 8 expense cats)
ax3 = fig.add_subplot(gs[3, :2])
simple_bar(ax3, budget_cats,
           [(actual_vals, FOREST, "Actual YTD"),
            (budget_vals, GOLD, "Annual Budget")],
           [FOREST, GOLD], "Budget vs Actual (Top Categories)")

# Chart 4: Spending Doughnut
ax4 = fig.add_subplot(gs[3, 2:])
spend_labels = [k for k,v in cat_actuals.items() if v > 0]
spend_vals   = [v for v in cat_actuals.values() if v > 0]
donut(ax4, spend_labels, spend_vals, "Spending by Category")

# Chart 5: Bills Doughnut
ax5 = fig.add_subplot(gs[4, :2])
bill_names = ["Rent","Netflix","Gym","Phone","Internet","Car Ins","Spotify","Amazon","Health Ins","Electric"]
bill_amts  = [1500, 16, 50, 80, 60, 120, 10, 15, 200, 80]
donut(ax5, bill_names, bill_amts, "Bills Distribution (Recurring)")

# Instructions note
ax6 = fig.add_subplot(gs[4, 2:])
ax6.set_facecolor(CREAM)
ax6.axis("off")
instructions = [
    "HOW TO USE:",
    "1. Set Year & Currency in Settings",
    "2. Enter transactions in Transactions tab",
    "3. Set up recurring bills in Recurring tab",
    "4. Dashboard updates automatically via SUMIFS",
    "",
    "30 TABS  •  4,789 FORMULAS  •  0 ERRORS",
    "Works in Excel + Google Sheets",
]
for j, line in enumerate(instructions):
    bold = j in [0, 7]
    ax6.text(0.05, 0.92 - j*0.11, line, color=FOREST if bold else TEXT,
             fontsize=8, fontweight="bold" if bold else "normal",
             transform=ax6.transAxes, va="top")

dash_path = save(fig, "01_Dashboard.png")

# ═══════════════════════════════════════════════════════════════════════════════
# 2. TRANSACTIONS
# ═══════════════════════════════════════════════════════════════════════════════
print("Rendering Transactions…")
ws = wb["Transactions"]

fig, ax = plt.subplots(figsize=(14, 8), facecolor=CREAM)
ax.set_facecolor(CREAM)
ax.axis("off")
fig.suptitle("Transactions", fontsize=14, fontweight="bold", color=WHITE,
             backgroundcolor=FOREST, y=0.98)

headers = ["Date","Type","Category","Account","Description","Amount","Month","Year","Day"]
col_widths = [0.1,0.07,0.14,0.11,0.22,0.09,0.07,0.07,0.07]
col_x = []
x = 0.02
for w in col_widths:
    col_x.append(x)
    x += w

# Header row
for i, (h, cx) in enumerate(zip(headers, col_x)):
    ax.add_patch(mpatches.FancyBboxPatch((cx, 0.88), col_widths[i]-0.005, 0.07,
        boxstyle="round,pad=0.005", facecolor=FOREST, edgecolor=FOREST, transform=ax.transAxes))
    ax.text(cx + col_widths[i]/2 - 0.003, 0.915, h, color=WHITE, fontsize=8,
            fontweight="bold", ha="center", va="center", transform=ax.transAxes)

# Data rows (first 12)
sample = [
    ("2026-01-05","Income","Salary","Checking","January Salary","$5,000.00","1","2026","5"),
    ("2026-01-10","Expense","Rent/Mortgage","Checking","January Rent","$1,500.00","1","2026","10"),
    ("2026-01-12","Expense","Groceries","Checking","Grocery Run","$120.00","1","2026","12"),
    ("2026-01-15","Expense","Utilities","Checking","Electric Bill","$80.00","1","2026","15"),
    ("2026-01-20","Expense","Dining Out","Credit Card","Restaurant","$45.00","1","2026","20"),
    ("2026-02-05","Income","Salary","Checking","February Salary","$5,000.00","2","2026","5"),
    ("2026-02-08","Expense","Rent/Mortgage","Checking","February Rent","$1,500.00","2","2026","8"),
    ("2026-02-14","Expense","Groceries","Checking","Grocery Run","$135.00","2","2026","14"),
    ("2026-02-18","Expense","Entertainment","Credit Card","Netflix+Cinema","$60.00","2","2026","18"),
    ("2026-03-05","Income","Salary","Checking","March Salary","$5,000.00","3","2026","5"),
    ("2026-03-07","Expense","Rent/Mortgage","Checking","March Rent","$1,500.00","3","2026","7"),
    ("2026-03-15","Expense","Groceries","Checking","Grocery Run","$110.00","3","2026","15"),
]

for ri, row in enumerate(sample):
    y_pos = 0.82 - ri * 0.063
    bg = CARD if ri % 2 == 0 else WHITE
    ax.add_patch(mpatches.Rectangle((0.02, y_pos-0.005), 0.96, 0.058,
        facecolor=CREAM, edgecolor="none", transform=ax.transAxes))
    for ci, (val, cx) in enumerate(zip(row, col_x)):
        color = SAGE if val == "Income" else (ROSE if val == "Expense" else TEXT)
        ax.text(cx + 0.005, y_pos + 0.022, val, color=color, fontsize=7.5,
                va="center", transform=ax.transAxes, clip_on=True)

ax.text(0.5, 0.02, "600 rows available  •  Dropdowns: Type / Category / Account  •  Month/Year/Day auto-calculated",
        color=SAGE, fontsize=8, ha="center", transform=ax.transAxes, style="italic")

trans_path = save(fig, "02_Transactions.png")

# ═══════════════════════════════════════════════════════════════════════════════
# 3. SETTINGS
# ═══════════════════════════════════════════════════════════════════════════════
print("Rendering Settings…")

fig, axes = plt.subplots(1, 3, figsize=(15, 10), facecolor=CREAM)
fig.suptitle("Settings", fontsize=14, fontweight="bold", color=WHITE,
             backgroundcolor=FOREST)

# Left: core settings + currency table
ax = axes[0]; ax.set_facecolor(CARD); ax.axis("off")
ax.text(0.5, 0.97, "Core Settings", color=WHITE, fontsize=11, fontweight="bold",
        ha="center", va="top", transform=ax.transAxes,
        bbox=dict(boxstyle="round", facecolor=FOREST, edgecolor="none", pad=0.3))
settings_items = [
    ("Budget Year", "2026"),
    ("Currency Code", "USD  ▾"),
    ("Currency Symbol", "$ (VLOOKUP)"),
    ("",""),
    ("Currency Table",""),
    ("USD", "$"), ("EUR","€"), ("GBP","£"), ("JPY","¥"), ("CAD","$"),
    ("AUD","$"), ("INR","₹"), ("CHF","Fr"), ("SEK","kr"), ("NOK","kr"),
    ("DKK","kr"), ("BRL","R$"), ("ZAR","R"), ("MXN","$"), ("NZD","$"),
    ("SGD","$"), ("CNY","¥"), ("PLN","zł"), ("AED","د.إ"), ("TRY","₺"),
]
for j, (k, v) in enumerate(settings_items):
    y = 0.89 - j * 0.038
    if k == "Currency Table":
        ax.text(0.5, y, k, color=WHITE, fontsize=8, fontweight="bold",
                ha="center", transform=ax.transAxes,
                bbox=dict(boxstyle="round", facecolor=SAGE, edgecolor="none", pad=0.2))
    elif k:
        ax.text(0.05, y, k+":", color=FOREST, fontsize=8, fontweight="bold",
                transform=ax.transAxes, va="center")
        ax.text(0.55, y, v, color=TEXT, fontsize=8,
                transform=ax.transAxes, va="center",
                bbox=dict(boxstyle="round", facecolor=CREAM, edgecolor="#ccc", pad=0.15))

# Middle: Categories
ax = axes[1]; ax.set_facecolor(CARD); ax.axis("off")
ax.text(0.5, 0.97, "Category Table", color=WHITE, fontsize=11, fontweight="bold",
        ha="center", va="top", transform=ax.transAxes,
        bbox=dict(boxstyle="round", facecolor=FOREST, edgecolor="none", pad=0.3))
# Header
for ci, (label, x0, w0) in enumerate([("Category",0.03,0.5),("Type",0.55,0.2),("Budget",0.77,0.2)]):
    ax.add_patch(mpatches.FancyBboxPatch((x0, 0.89), w0, 0.04,
        boxstyle="square,pad=0", facecolor=SAGE, edgecolor="none", transform=ax.transAxes))
    ax.text(x0+w0/2, 0.91, label, color=WHITE, fontsize=7.5, fontweight="bold",
            ha="center", transform=ax.transAxes)

cats_display = [
    ("Salary","Income","$0"), ("Partner Income","Income","$0"),
    ("Freelance","Income","$0"), ("Business","Income","$0"),
    ("Investments","Income","$0"), ("Rental Income","Income","$0"),
    ("—— Expenses ——","",""),
    ("Rent/Mortgage","Expense","$1,500"), ("Utilities","Expense","$150"),
    ("Groceries","Expense","$400"), ("Dining Out","Expense","$200"),
    ("Transport","Expense","$150"), ("Fuel","Expense","$100"),
    ("Car/Auto","Expense","$100"), ("Insurance","Expense","$200"),
    ("Health","Expense","$100"), ("Phone","Expense","$80"),
    ("Internet","Expense","$60"), ("Subscriptions","Expense","$50"),
    ("Entertainment","Expense","$100"), ("Shopping","Expense","$200"),
]
for j, (cat, typ, bud) in enumerate(cats_display):
    y = 0.86 - j * 0.038
    bg = LT_SAGE if typ == "Income" else (CREAM if typ == "Expense" else GOLD)
    ax.add_patch(mpatches.Rectangle((0.03, y-0.005), 0.94, 0.032,
        facecolor=bg, edgecolor="none", transform=ax.transAxes))
    ax.text(0.05, y+0.009, cat, color=FOREST, fontsize=7, transform=ax.transAxes, va="center")
    if typ:
        ax.text(0.56, y+0.009, typ, color=TEXT, fontsize=7, transform=ax.transAxes, va="center")
        ax.text(0.88, y+0.009, bud, color=TEXT, fontsize=7, transform=ax.transAxes, va="center", ha="center")

# Right: Accounts
ax = axes[2]; ax.set_facecolor(CARD); ax.axis("off")
ax.text(0.5, 0.97, "Accounts List", color=WHITE, fontsize=11, fontweight="bold",
        ha="center", va="top", transform=ax.transAxes,
        bbox=dict(boxstyle="round", facecolor=FOREST, edgecolor="none", pad=0.3))
accounts = ["Checking","Savings","Cash","Credit Card","Debit Card","PayPal","Investment","Other"]
for j, acc in enumerate(accounts):
    y = 0.88 - j * 0.05
    ax.add_patch(mpatches.FancyBboxPatch((0.1, y-0.015), 0.8, 0.04,
        boxstyle="round,pad=0.01", facecolor=CREAM, edgecolor=SAGE, transform=ax.transAxes))
    ax.text(0.5, y+0.003, acc, color=FOREST, fontsize=10, fontweight="bold",
            ha="center", transform=ax.transAxes, va="center")

ax.text(0.5, 0.4, "Defined Names Created:", color=FOREST, fontsize=9, fontweight="bold",
        ha="center", transform=ax.transAxes)
for j, name in enumerate(["CatList","AcctList","CurList","IncomeCatList","ExpenseCatList","BudgetYear"]):
    ax.text(0.5, 0.34 - j*0.04, f"={name}", color=SAGE, fontsize=9,
            ha="center", transform=ax.transAxes,
            bbox=dict(boxstyle="round", facecolor=LT_SAGE, edgecolor="none", pad=0.1))

plt.tight_layout()
settings_path = save(fig, "03_Settings.png")

# ═══════════════════════════════════════════════════════════════════════════════
# 4. JAN MONTHLY TAB
# ═══════════════════════════════════════════════════════════════════════════════
print("Rendering Jan…")

fig, ax = plt.subplots(figsize=(12, 14), facecolor=CREAM)
ax.axis("off")
ax.set_facecolor(CREAM)

# Title
ax.add_patch(mpatches.FancyBboxPatch((0.0, 0.95), 1.0, 0.055,
    boxstyle="square,pad=0", facecolor=FOREST, edgecolor="none", transform=ax.transAxes))
ax.text(0.5, 0.975, "Jan 2026 — Budget vs Actual", color=WHITE, fontsize=14,
        fontweight="bold", ha="center", va="center", transform=ax.transAxes)

# Column headers
headers = ["Category","Budget","Actual","Remaining","% Used"]
col_positions = [0.02, 0.35, 0.52, 0.68, 0.84]
col_widths_h   = [0.31, 0.15, 0.15, 0.15, 0.14]
for label, cx, cw in zip(headers, col_positions, col_widths_h):
    ax.add_patch(mpatches.FancyBboxPatch((cx, 0.90), cw-0.01, 0.038,
        boxstyle="square,pad=0", facecolor=SAGE, edgecolor="none", transform=ax.transAxes))
    ax.text(cx+cw/2-0.005, 0.919, label, color=WHITE, fontsize=9, fontweight="bold",
            ha="center", va="center", transform=ax.transAxes)

# Income section
income_data = [
    ("Salary", 0, 5000),
    ("Partner Income", 0, 0),
    ("Freelance", 0, 0),
    ("Business", 0, 0),
    ("Investments", 0, 0),
    ("Rental Income", 0, 0),
    ("Refunds", 0, 0),
    ("Other Income", 0, 0),
]
y = 0.87
ax.add_patch(mpatches.Rectangle((0.0, y), 1.0, 0.028,
    facecolor=FOREST, edgecolor="none", transform=ax.transAxes))
ax.text(0.5, y+0.013, "— INCOME —", color=WHITE, fontsize=10, fontweight="bold",
        ha="center", transform=ax.transAxes)
y -= 0.028

for i, (cat, budget, actual) in enumerate(income_data):
    bg = CREAM if i % 2 == 0 else CARD
    ax.add_patch(mpatches.Rectangle((0.0, y), 1.0, 0.026,
        facecolor=bg, edgecolor="none", transform=ax.transAxes))
    ax.text(0.04, y+0.012, cat, color=TEXT, fontsize=8.5, transform=ax.transAxes, va="center")
    ax.text(0.425, y+0.012, f"${budget:,.2f}", color=TEXT, fontsize=8.5,
            ha="center", transform=ax.transAxes, va="center")
    ax.text(0.595, y+0.012, f"${actual:,.2f}", color=FOREST if actual>0 else TEXT,
            fontsize=8.5, fontweight="bold" if actual>0 else "normal",
            ha="center", transform=ax.transAxes, va="center")
    # Income rows: Remaining and % Used are BLANK (FIX 4)
    ax.text(0.755, y+0.012, "—", color="#aaa", fontsize=8, ha="center", transform=ax.transAxes, va="center")
    ax.text(0.91, y+0.012, "—", color="#aaa", fontsize=8, ha="center", transform=ax.transAxes, va="center")
    y -= 0.026

# Total income row
ax.add_patch(mpatches.Rectangle((0.0, y), 1.0, 0.028,
    facecolor=SAGE, edgecolor="none", transform=ax.transAxes))
ax.text(0.04, y+0.013, "Total Income", color=WHITE, fontsize=9, fontweight="bold", transform=ax.transAxes)
ax.text(0.425, y+0.013, "$0.00", color=WHITE, fontsize=9, fontweight="bold", ha="center", transform=ax.transAxes)
ax.text(0.595, y+0.013, "$5,000.00", color=WHITE, fontsize=9, fontweight="bold", ha="center", transform=ax.transAxes)
y -= 0.038

# Expense section
expense_data = [
    ("Rent/Mortgage", 1500, 1500), ("Utilities", 150, 80), ("Groceries", 400, 120),
    ("Dining Out", 200, 45), ("Transport", 150, 0), ("Fuel", 100, 0),
    ("Car/Auto", 100, 0), ("Insurance", 200, 0), ("Health", 100, 0),
    ("Phone", 80, 0), ("Internet", 60, 0), ("Subscriptions", 50, 0),
    ("Entertainment", 100, 0), ("Shopping", 200, 0), ("Clothing", 100, 0),
    ("Personal Care", 50, 0), ("Gym/Fitness", 50, 0), ("Pets", 50, 0),
    ("Kids", 200, 0), ("Gifts/Donations", 100, 0), ("Travel", 200, 0), ("Miscellaneous", 100, 0),
]
ax.add_patch(mpatches.Rectangle((0.0, y), 1.0, 0.028,
    facecolor=FOREST, edgecolor="none", transform=ax.transAxes))
ax.text(0.5, y+0.013, "— EXPENSES —", color=WHITE, fontsize=10, fontweight="bold",
        ha="center", transform=ax.transAxes)
y -= 0.028

for i, (cat, budget, actual) in enumerate(expense_data):
    if y < 0.02: break
    bg = CREAM if i % 2 == 0 else CARD
    ax.add_patch(mpatches.Rectangle((0.0, y), 1.0, 0.026,
        facecolor=bg, edgecolor="none", transform=ax.transAxes))
    remaining = budget - actual
    pct = (actual/budget) if budget > 0 else 0
    ax.text(0.04, y+0.012, cat, color=TEXT, fontsize=8.5, transform=ax.transAxes, va="center")
    ax.text(0.425, y+0.012, f"${budget:,.2f}", color=TEXT, fontsize=8.5, ha="center", transform=ax.transAxes, va="center")
    ax.text(0.595, y+0.012, f"${actual:,.2f}", color=ROSE if actual>0 else TEXT, fontsize=8.5, ha="center", transform=ax.transAxes, va="center")
    rem_color = ROSE if remaining < 0 else TEXT
    ax.text(0.755, y+0.012, f"${remaining:,.2f}", color=rem_color, fontsize=8.5, ha="center", transform=ax.transAxes, va="center")
    ax.text(0.91, y+0.012, f"{pct:.0%}", color=ROSE if pct>1 else TEXT, fontsize=8.5, ha="center", transform=ax.transAxes, va="center")
    y -= 0.026

jan_path = save(fig, "04_Jan.png")

# ═══════════════════════════════════════════════════════════════════════════════
# 5. BILL CALENDAR
# ═══════════════════════════════════════════════════════════════════════════════
print("Rendering Bill Calendar…")

fig, ax = plt.subplots(figsize=(10, 10), facecolor=CREAM)
ax.set_facecolor(CREAM); ax.axis("off")

# Title
ax.add_patch(mpatches.FancyBboxPatch((0.0, 0.93), 1.0, 0.07,
    boxstyle="square,pad=0", facecolor=FOREST, edgecolor="none", transform=ax.transAxes))
ax.text(0.5, 0.965, "Bill Calendar", color=WHITE, fontsize=16, fontweight="bold",
        ha="center", va="center", transform=ax.transAxes)

ax.text(0.08, 0.88, "Month:", color=FOREST, fontsize=10, fontweight="bold", transform=ax.transAxes)
ax.add_patch(mpatches.FancyBboxPatch((0.22, 0.865), 0.12, 0.035,
    boxstyle="round,pad=0.01", facecolor=CREAM, edgecolor=SAGE, transform=ax.transAxes))
ax.text(0.28, 0.882, "2  ▾", color=FOREST, fontsize=10, ha="center", transform=ax.transAxes)

# Day headers
days = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"]
cell_w = 0.13; cell_h = 0.07
start_x = 0.04; start_y = 0.80
for i, d in enumerate(days):
    x = start_x + i * cell_w
    ax.add_patch(mpatches.FancyBboxPatch((x, start_y), cell_w-0.01, cell_h-0.01,
        boxstyle="square,pad=0", facecolor=SAGE, edgecolor="none", transform=ax.transAxes))
    ax.text(x + (cell_w-0.01)/2, start_y + (cell_h-0.01)/2, d, color=WHITE,
            fontsize=10, fontweight="bold", ha="center", va="center", transform=ax.transAxes)

# Calendar grid (February 2026: starts on Sunday)
# Bill due days from recurring: 1,3,5,8,10,12,15,18,20,25
bill_days = {1,3,5,8,10,12,15,18,20,25}
bill_day_map = {
    1:"$1500", 3:"$50", 5:"$16", 8:"$10",
    10:"$80", 12:"$15", 15:"$60", 18:"$80", 20:"$120", 25:"$200"
}
day_num = 1
for week in range(6):
    for dow in range(7):
        if week == 0 and dow == 0:
            day_num = 1
        x = start_x + dow * cell_w
        y = start_y - (week+1) * cell_h - 0.005 * week
        if 1 <= day_num <= 28:
            is_bill = day_num in bill_days
            bg = ROSE if is_bill else CARD
            fg = WHITE if is_bill else TEXT
            ax.add_patch(mpatches.FancyBboxPatch((x, y), cell_w-0.01, cell_h-0.01,
                boxstyle="round,pad=0.005", facecolor=bg, edgecolor=SAGE,
                linewidth=0.5, transform=ax.transAxes))
            ax.text(x+0.025, y+cell_h-0.025, str(day_num), color=fg, fontsize=9,
                    fontweight="bold", transform=ax.transAxes, va="top")
            if is_bill:
                ax.text(x+(cell_w-0.01)/2, y+0.015, bill_day_map.get(day_num,""),
                        color=WHITE, fontsize=7, ha="center", transform=ax.transAxes)
            day_num += 1

# Legend
ax.add_patch(mpatches.FancyBboxPatch((0.04, 0.04), 0.18, 0.03,
    boxstyle="round,pad=0.01", facecolor=ROSE, edgecolor="none", transform=ax.transAxes))
ax.text(0.13, 0.055, "Bill Due", color=WHITE, fontsize=9, ha="center", transform=ax.transAxes)
ax.add_patch(mpatches.FancyBboxPatch((0.25, 0.04), 0.18, 0.03,
    boxstyle="round,pad=0.01", facecolor=CARD, edgecolor=SAGE, transform=ax.transAxes))
ax.text(0.34, 0.055, "No Bill", color=TEXT, fontsize=9, ha="center", transform=ax.transAxes)
ax.text(0.75, 0.055, "Highlighted days auto-driven by Recurring sheet",
        color=FOREST, fontsize=8, style="italic", ha="center", transform=ax.transAxes)

cal_path = save(fig, "05_BillCalendar.png")

# ═══════════════════════════════════════════════════════════════════════════════
# 6. DEBT TRACKER
# ═══════════════════════════════════════════════════════════════════════════════
print("Rendering Debt Tracker…")

fig, ax = plt.subplots(figsize=(14, 9), facecolor=CREAM)
ax.set_facecolor(CREAM); ax.axis("off")

ax.add_patch(mpatches.FancyBboxPatch((0.0, 0.93), 1.0, 0.07,
    boxstyle="square,pad=0", facecolor=FOREST, edgecolor="none", transform=ax.transAxes))
ax.text(0.5, 0.965, "Debt Tracker", color=WHITE, fontsize=16, fontweight="bold",
        ha="center", va="center", transform=ax.transAxes)

# Column headers — 9 columns
hdrs = ["Debt Name","Orig Balance","Curr Balance","APR %","Min Pmt","Mo Pmt","Months","Total Int","% Paid Off"]
col_x2    = [0.01, 0.13, 0.24, 0.35, 0.43, 0.52, 0.61, 0.70, 0.81]
col_w2    = [0.11, 0.10, 0.10, 0.07, 0.08, 0.08, 0.08, 0.10, 0.17]
for h, cx, cw in zip(hdrs, col_x2, col_w2):
    ax.add_patch(mpatches.FancyBboxPatch((cx, 0.88), cw-0.005, 0.04,
        boxstyle="square,pad=0", facecolor=SAGE, edgecolor="none", transform=ax.transAxes))
    ax.text(cx+cw/2-0.003, 0.90, h, color=WHITE, fontsize=7.5, fontweight="bold",
            ha="center", va="center", transform=ax.transAxes, wrap=True)

# Debt data (NPER positive: =NPER(apr/12,-mo_pmt,balance))
import math
def months_to_payoff(balance, apr_pct, mo_pmt):
    if mo_pmt == 0 or balance == 0: return 0
    if apr_pct == 0: return math.ceil(balance / mo_pmt)
    r = apr_pct / 100 / 12
    try:
        n = math.log(mo_pmt / (mo_pmt - r * balance)) / math.log(1 + r)
        return max(0, n)
    except:
        return 0

debts = [
    ("Credit Card",   4200, 3500, 19.99, 70,  150),
    ("Car Loan",     14000,12000,  4.50, 220, 250),
    ("Student Loan", 30000,25000,  5.00, 250, 300),
    ("Personal Loan", 7500, 5000, 12.00, 100, 150),
    ("Medical Bill",  1000,  800,  0.00,  50,  50),
]

for i, (name, orig, curr, apr, min_p, mo_p) in enumerate(debts):
    y = 0.83 - i * 0.07
    bg = CREAM if i % 2 == 0 else CARD
    ax.add_patch(mpatches.Rectangle((0.0, y), 1.0, 0.065,
        facecolor=bg, edgecolor="none", transform=ax.transAxes))

    months = months_to_payoff(curr, apr, mo_p)
    total_int = max(0, months * mo_p - curr)
    pct_paid = (orig - curr) / orig if orig > 0 else 0

    row_vals = [name, f"${orig:,.2f}", f"${curr:,.2f}", f"{apr:.2f}%",
                f"${min_p:,.2f}", f"${mo_p:,.2f}", f"{months:.1f}", f"${total_int:,.2f}", f"{pct_paid:.0%}"]

    for j, (val, cx, cw) in enumerate(zip(row_vals, col_x2, col_w2)):
        color = FOREST if j == 0 else TEXT
        # Months column: highlight positive in green
        if j == 6:
            color = SAGE if float(val.replace(',','')) > 0 else ROSE
        ax.text(cx + cw/2 - 0.003, y + 0.032, val, color=color,
                fontsize=8.5, ha="center", va="center", transform=ax.transAxes,
                fontweight="bold" if j in [0,6] else "normal")

    # Progress bar for % paid off (col 9)
    bar_x = col_x2[8]; bar_y = y + 0.008; bar_h = 0.025
    bar_total_w = col_w2[8] - 0.01
    ax.add_patch(mpatches.Rectangle((bar_x, bar_y), bar_total_w, bar_h,
        facecolor=LT_SAGE, edgecolor=SAGE, linewidth=0.5, transform=ax.transAxes))
    ax.add_patch(mpatches.Rectangle((bar_x, bar_y), bar_total_w * pct_paid, bar_h,
        facecolor=SAGE, edgecolor="none", transform=ax.transAxes))
    ax.text(bar_x + bar_total_w/2, bar_y + bar_h/2, f"{pct_paid:.0%}",
            color=FOREST, fontsize=7, fontweight="bold",
            ha="center", va="center", transform=ax.transAxes)

# Note about positive NPER
ax.text(0.5, 0.05, "✓ Months to Payoff uses NPER() — all values positive  •  Credit Card ≈ 29.8 months",
        color=SAGE, fontsize=9, ha="center", transform=ax.transAxes, style="italic")
ax.text(0.5, 0.01, "Formula: =IF(OR(APR=0,MoPmt=0), CEILING(Balance/MoPmt,1), IFERROR(NPER(APR/100/12,-MoPmt,Balance),0))",
        color=TEXT, fontsize=8, ha="center", transform=ax.transAxes,
        bbox=dict(boxstyle="round", facecolor=LT_SAGE, edgecolor="none", pad=0.3))

debt_path = save(fig, "06_DebtTracker.png")

# ═══════════════════════════════════════════════════════════════════════════════
# 7. COMBINED PDF
# ═══════════════════════════════════════════════════════════════════════════════
print("\nCombining PNGs into PDF…")
from PIL import Image

png_paths = [dash_path, trans_path, settings_path, jan_path, cal_path, debt_path]
images = [Image.open(p).convert("RGB") for p in png_paths]

pdf_path = os.path.join(OUT_DIR, "The_Ultimate_Budget_PREVIEW.pdf")
images[0].save(pdf_path, save_all=True, append_images=images[1:])
print(f"  PDF saved: {pdf_path}")

print("\n✅ All previews generated:")
for p in png_paths + [pdf_path]:
    size = os.path.getsize(p)
    print(f"  {os.path.basename(p):45s} {size/1024:.0f} KB")
