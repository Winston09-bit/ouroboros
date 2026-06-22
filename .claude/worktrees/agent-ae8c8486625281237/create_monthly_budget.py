"""
create_monthly_budget.py
Builds ultimate_monthly_budget.xlsx with openpyxl native charts (no xlwings).
"""

import os
from openpyxl import Workbook
from openpyxl.styles import (
    Font, PatternFill, Alignment, Border, Side, numbers
)
from openpyxl.utils import get_column_letter, column_index_from_string
from openpyxl.chart import BarChart, Reference, Series
from openpyxl.chart.series import DataPoint
from openpyxl.chart import DoughnutChart
from openpyxl.chart.label import DataLabelList

# ── Palette ──────────────────────────────────────────────────────────────────
ROSE_DARK   = "7D3C4E"
ROSE_MID    = "A85470"
ROSE_LIGHT  = "E8B4C0"
ROSE_PALE   = "F9E8ED"
SAGE_DARK   = "4A6741"
SAGE_LIGHT  = "C8DDB8"
SAGE_PALE   = "EEF4E8"
CREAM       = "FAF6F0"
GOLD        = "C4A882"
WHITE       = "FFFFFF"
CHARCOAL    = "2C2C2C"
INPUT_BLUE  = "1A4FA0"
WARM_GRAY   = "8C8279"
CHART_ROSE  = "C97B8E"
CHART_GREEN = "6B9E63"
CHART_LINE  = "7D3C4E"
CHART_DONUT = ["7D3C4E","A85470","C97B8E","E8B4C0","F9E8ED","4A6741","6B9E63","C8DDB8"]

MONEY_FMT = '#,##0;[RED]-#,##0;"-"'
PCT_FMT   = '0.0%;[RED]-0.0%'

MONTHS = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"]

INCOME_SOURCES = [
    "Primary Salary","Freelance","Rental Income",
    "Dividends","Side Business","Other Income"
]

EXPENSE_CATS = {
    "HOUSING":              ["Rent/Mortgage","Electricity","Water","Internet","Home Insurance","Maintenance"],
    "TRANSPORT":            ["Car Payment","Fuel","Car Insurance","Parking","Public Transport","Ride Share"],
    "FOOD & DINING":        ["Groceries","Restaurants","Coffee Shops","Food Delivery","Work Lunch","Other"],
    "HEALTH & WELLNESS":    ["Health Insurance","Gym Membership","Pharmacy","Doctor Visits","Dental","Mental Health"],
    "LIFESTYLE":            ["Streaming Services","Entertainment","Hobbies","Personal Care","Books","Clothing"],
    "CHILDREN & FAMILY":    ["Childcare","School Fees","Activities","Toys & Gifts","Family Outings","Other"],
    "SAVINGS & INVESTMENTS":["Emergency Fund","Pension/IRA","Stocks & ETFs","Vacation Fund","Home Fund","Other"],
    "DEBT PAYMENTS":        ["Credit Card 1","Credit Card 2","Student Loan","Car Loan","Personal Loan","Other"],
}

SAVINGS_GOALS = [
    ("Emergency Fund",      50000),
    ("Vacation-Japan",      25000),
    ("Home Down Payment",  250000),
    ("New Car",             80000),
    ("Retirement Boost",   100000),
    ("Education Fund",      40000),
    ("Wedding Fund",        60000),
    ("Home Renovation",     35000),
]

# ── Style helpers ─────────────────────────────────────────────────────────────
def fill(hex_color):
    return PatternFill("solid", fgColor=hex_color)

def font(name="Calibri", size=11, bold=False, italic=False, color="000000"):
    return Font(name=name, size=size, bold=bold, italic=italic, color=color)

def align(h="left", v="center", wrap=False):
    return Alignment(horizontal=h, vertical=v, wrap_text=wrap)

def thin_border():
    s = Side(style="thin", color="D0D0D0")
    return Border(left=s, right=s, top=s, bottom=s)

def set_cell(ws, row, col, value=None, bg=None, fnt=None, aln=None, fmt=None, brd=None):
    cell = ws.cell(row=row, column=col)
    if value is not None:
        cell.value = value
    if bg:
        cell.fill = fill(bg)
    if fnt:
        cell.font = fnt
    if aln:
        cell.alignment = aln
    if fmt:
        cell.number_format = fmt
    if brd:
        cell.border = brd
    return cell

def merge_row(ws, row, col_start, col_end, value, bg, fnt, aln=None):
    ws.merge_cells(start_row=row, start_column=col_start,
                   end_row=row, end_column=col_end)
    cell = ws.cell(row=row, column=col_start)
    cell.value = value
    cell.fill = fill(bg)
    cell.font = fnt
    cell.alignment = aln or align("center", "center")
    return cell

def section_header(ws, row, ncols, text, bg, fnt):
    merge_row(ws, row, 1, ncols, text, bg, fnt, align("left","center"))
    for c in range(2, ncols+1):
        ws.cell(row=row, column=c).fill = fill(bg)

# ── Monthly sheet builder ─────────────────────────────────────────────────────
def build_monthly(wb, mo_name, rows_tracker):
    ws = wb[mo_name]
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "A4"

    NCOLS = 7
    col_widths = [22,22,13,13,13,10,18]
    for i, w in enumerate(col_widths, 1):
        ws.column_dimensions[get_column_letter(i)].width = w

    mo = {}

    # Row 1: Title
    r = 1
    merge_row(ws, r, 1, NCOLS, f"{mo_name.upper()} 2026 · MONTHLY BUDGET",
              ROSE_DARK, font(size=15, bold=True, color=WHITE))
    ws.row_dimensions[r].height = 30

    # Row 2: Subtitle
    r = 2
    merge_row(ws, r, 1, NCOLS, "Track your income, expenses and cash flow",
              GOLD, font(size=10, italic=True, color=CHARCOAL))
    ws.row_dimensions[r].height = 20

    # Row 3: Column headers
    r = 3
    headers = ["CATEGORY","SUBCATEGORY","BUDGETED","ACTUAL","DIFFERENCE","% USED","NOTES"]
    for c, h in enumerate(headers, 1):
        set_cell(ws, r, c, h, bg=ROSE_DARK,
                 fnt=font(bold=True, color=WHITE),
                 aln=align("center","center"))
    ws.row_dimensions[r].height = 20

    r = 4

    # ── Income section ────────────────────────────────────────────────────────
    mo["income_section"] = r
    section_header(ws, r, NCOLS, "INCOME", SAGE_DARK, font(bold=True, color=WHITE))
    ws.row_dimensions[r].height = 18
    r += 1

    mo["income_start"] = r
    for i, src in enumerate(INCOME_SOURCES):
        bg = ROSE_PALE if i % 2 == 0 else WHITE
        set_cell(ws, r, 1, "Income", bg=bg, fnt=font(), aln=align())
        set_cell(ws, r, 2, src,      bg=bg, fnt=font(color=INPUT_BLUE), aln=align())
        for c in range(3, 7):
            set_cell(ws, r, c, 0 if c < 5 else None,
                     bg=bg, fnt=font(),
                     aln=align("right"),
                     fmt=MONEY_FMT if c < 6 else PCT_FMT,
                     brd=thin_border())
        # Difference = Actual - Budgeted
        B = ws.cell(row=r, column=3).coordinate
        D = ws.cell(row=r, column=4).coordinate
        ws.cell(row=r, column=5).value = f"={D}-{B}"
        ws.cell(row=r, column=5).number_format = MONEY_FMT
        # % used
        ws.cell(row=r, column=6).value = f"=IF({B}=0,0,{D}/{B})"
        ws.cell(row=r, column=6).number_format = PCT_FMT
        set_cell(ws, r, 7, "", bg=bg)
        ws.row_dimensions[r].height = 16
        r += 1

    mo["income_end"] = r - 1

    # Income total
    mo["income_total"] = r
    merge_row(ws, r, 1, 2, "TOTAL INCOME", SAGE_LIGHT, font(bold=True, color=CHARCOAL))
    for c in range(3, 7):
        start = get_column_letter(c) + str(mo["income_start"])
        end   = get_column_letter(c) + str(mo["income_end"])
        val = f"=SUM({start}:{end})"
        set_cell(ws, r, c, val, bg=SAGE_LIGHT, fnt=font(bold=True),
                 aln=align("right"),
                 fmt=MONEY_FMT if c < 6 else PCT_FMT)
    set_cell(ws, r, 7, "", bg=SAGE_LIGHT)
    ws.row_dimensions[r].height = 18
    r += 1

    # ── Expense categories ────────────────────────────────────────────────────
    expense_total_rows = []
    for cat_name, subs in EXPENSE_CATS.items():
        cat_key = cat_name.lower().replace(" ","_").replace("&","").replace("__","_")
        mo[f"{cat_key}_section"] = r
        section_header(ws, r, NCOLS, cat_name, ROSE_MID, font(bold=True, color=WHITE))
        ws.row_dimensions[r].height = 18
        r += 1

        sub_start = r
        for i, sub in enumerate(subs):
            bg = ROSE_PALE if i % 2 == 0 else WHITE
            set_cell(ws, r, 1, cat_name, bg=bg, fnt=font(), aln=align())
            set_cell(ws, r, 2, sub,      bg=bg, fnt=font(color=INPUT_BLUE), aln=align())
            for c in [3, 4]:
                set_cell(ws, r, c, 0, bg=bg, fnt=font(), aln=align("right"), fmt=MONEY_FMT, brd=thin_border())
            B = ws.cell(row=r, column=3).coordinate
            D = ws.cell(row=r, column=4).coordinate
            ws.cell(row=r, column=5).value = f"={D}-{B}"
            ws.cell(row=r, column=5).number_format = MONEY_FMT
            ws.cell(row=r, column=5).fill = fill(bg)
            ws.cell(row=r, column=6).value = f"=IF({B}=0,0,{D}/{B})"
            ws.cell(row=r, column=6).number_format = PCT_FMT
            ws.cell(row=r, column=6).fill = fill(bg)
            set_cell(ws, r, 7, "", bg=bg)
            ws.row_dimensions[r].height = 16
            r += 1

        sub_end = r - 1
        mo[f"{cat_key}_total"] = r
        expense_total_rows.append(r)
        merge_row(ws, r, 1, 2, f"SUBTOTAL · {cat_name}", ROSE_LIGHT, font(bold=True, color=CHARCOAL))
        for c in range(3, 7):
            start = get_column_letter(c) + str(sub_start)
            end   = get_column_letter(c) + str(sub_end)
            set_cell(ws, r, c, f"=SUM({start}:{end})",
                     bg=ROSE_LIGHT, fnt=font(bold=True),
                     aln=align("right"),
                     fmt=MONEY_FMT if c < 6 else PCT_FMT)
        set_cell(ws, r, 7, "", bg=ROSE_LIGHT)
        ws.row_dimensions[r].height = 18
        r += 1

    # Grand total expenses
    mo["grand_total"] = r
    merge_row(ws, r, 1, 2, "GRAND TOTAL EXPENSES", ROSE_DARK, font(bold=True, color=WHITE))
    for c in range(3, 7):
        refs = "+".join(get_column_letter(c)+str(er) for er in expense_total_rows)
        set_cell(ws, r, c, f"={refs}",
                 bg=ROSE_DARK, fnt=font(bold=True, color=WHITE),
                 aln=align("right"),
                 fmt=MONEY_FMT if c < 6 else PCT_FMT)
    set_cell(ws, r, 7, "", bg=ROSE_DARK)
    ws.row_dimensions[r].height = 20
    r += 1

    # Net cash flow
    mo["net_flow"] = r
    merge_row(ws, r, 1, 2, "NET CASH FLOW", SAGE_DARK, font(bold=True, color=WHITE))
    it = mo["income_total"]
    gt = mo["net_flow"] - 1
    for c in [3, 4]:
        ic = get_column_letter(c)
        set_cell(ws, r, c, f"={ic}{it}-{ic}{gt}",
                 bg=SAGE_DARK, fnt=font(bold=True, color=WHITE),
                 aln=align("right"), fmt=MONEY_FMT)
    set_cell(ws, r, 5, f"=D{r}-C{r}", bg=SAGE_DARK, fnt=font(bold=True, color=WHITE), aln=align("right"), fmt=MONEY_FMT)
    set_cell(ws, r, 6, f"=IF(C{r}=0,0,D{r}/C{r})", bg=SAGE_DARK, fnt=font(bold=True, color=WHITE), aln=align("right"), fmt=PCT_FMT)
    set_cell(ws, r, 7, "", bg=SAGE_DARK)
    ws.row_dimensions[r].height = 20

    rows_tracker[mo_name] = mo
    return mo


# ── Savings Tracker ──────────────────────────────────────────────────────────
def build_savings_tracker(wb, rows_tracker):
    ws = wb["Savings Tracker"]
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "A4"

    NCOLS = 8
    col_widths = [22, 14, 14, 14, 14, 10, 20, 12]
    for i, w in enumerate(col_widths, 1):
        ws.column_dimensions[get_column_letter(i)].width = w

    r = 1
    merge_row(ws, r, 1, NCOLS, "SAVINGS TRACKER · 2026",
              ROSE_DARK, font(size=15, bold=True, color=WHITE))
    ws.row_dimensions[r].height = 30

    r = 2
    merge_row(ws, r, 1, NCOLS, "Monitor progress toward your financial goals",
              GOLD, font(size=10, italic=True, color=CHARCOAL))
    ws.row_dimensions[r].height = 20

    r = 3
    hdrs = ["SAVINGS GOAL","TARGET","SAVED SO FAR","MONTHLY CONTRIB","TARGET DATE","% COMPLETE","PROGRESS","PRIORITY"]
    for c, h in enumerate(hdrs, 1):
        set_cell(ws, r, c, h, bg=ROSE_DARK, fnt=font(bold=True, color=WHITE), aln=align("center","center"))
    ws.row_dimensions[r].height = 20

    r = 4
    for i, (name, target) in enumerate(SAVINGS_GOALS):
        bg = ROSE_PALE if i % 2 == 0 else SAGE_PALE
        set_cell(ws, r, 1, name,   bg=bg, fnt=font(bold=True), aln=align())
        set_cell(ws, r, 2, target, bg=bg, fnt=font(color=INPUT_BLUE), aln=align("right"), fmt=MONEY_FMT)
        set_cell(ws, r, 3, 0,      bg=bg, fnt=font(color=INPUT_BLUE), aln=align("right"), fmt=MONEY_FMT)
        set_cell(ws, r, 4, 0,      bg=bg, fnt=font(color=INPUT_BLUE), aln=align("right"), fmt=MONEY_FMT)
        set_cell(ws, r, 5, "2027-12-31", bg=bg, fnt=font(color=INPUT_BLUE), aln=align("center"))
        # % complete
        set_cell(ws, r, 6, f"=IF(B{r}=0,0,C{r}/B{r})",
                 bg=bg, fnt=font(), aln=align("center"), fmt=PCT_FMT)
        # Progress bar
        set_cell(ws, r, 7,
                 f'=REPT(CHAR(9608),ROUND(MIN(F{r},1)*20,0))&REPT(CHAR(9617),20-ROUND(MIN(F{r},1)*20,0))',
                 bg=bg, fnt=font(color=ROSE_DARK), aln=align())
        set_cell(ws, r, 8, "High", bg=bg, fnt=font(), aln=align("center"))
        ws.row_dimensions[r].height = 18
        r += 1

    # Totals row
    merge_row(ws, r, 1, 1, "TOTAL", ROSE_DARK, font(bold=True, color=WHITE))
    for c in [2, 3, 4]:
        col = get_column_letter(c)
        set_cell(ws, r, c, f"=SUM({col}4:{col}{r-1})",
                 bg=ROSE_DARK, fnt=font(bold=True, color=WHITE),
                 aln=align("right"), fmt=MONEY_FMT)
    for c in [5, 6, 7, 8]:
        set_cell(ws, r, c, "", bg=ROSE_DARK)
    ws.row_dimensions[r].height = 20


# ── Annual Budget ─────────────────────────────────────────────────────────────
def build_annual(wb, rows_tracker):
    ws = wb["Annual Budget"]
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "B4"

    # Cols: LINE ITEM + Jan..Dec + ANNUAL TOTAL + ANNUAL BUDGET + VARIANCE
    NCOLS = 1 + 12 + 3
    col_widths = [22] + [10]*12 + [13, 13, 13]
    for i, w in enumerate(col_widths, 1):
        ws.column_dimensions[get_column_letter(i)].width = w

    r = 1
    merge_row(ws, r, 1, NCOLS, "ANNUAL BUDGET OVERVIEW · 2026",
              ROSE_DARK, font(size=15, bold=True, color=WHITE))
    ws.row_dimensions[r].height = 30

    r = 2
    merge_row(ws, r, 1, NCOLS, "Yearly summary across all months",
              GOLD, font(size=10, italic=True, color=CHARCOAL))
    ws.row_dimensions[r].height = 20

    r = 3
    hdrs = ["LINE ITEM"] + MONTHS + ["ANNUAL TOTAL", "ANNUAL BUDGET", "VARIANCE"]
    for c, h in enumerate(hdrs, 1):
        set_cell(ws, r, c, h, bg=ROSE_DARK, fnt=font(bold=True, color=WHITE), aln=align("center","center"))
    ws.row_dimensions[r].height = 20

    r = 4

    def annual_row(ws, r, label, bg, fnt_style, get_cell_ref):
        set_cell(ws, r, 1, label, bg=bg, fnt=fnt_style, aln=align())
        month_cols = []
        for mi, mo_name in enumerate(MONTHS, 2):
            ref = get_cell_ref(mo_name)
            set_cell(ws, r, mi, f"={ref}", bg=bg, fnt=fnt_style, aln=align("right"), fmt=MONEY_FMT)
            month_cols.append(get_column_letter(mi) + str(r))
        # Annual total (sum of B..M in this row)
        total_col = 1 + 12 + 1  # col 14
        set_cell(ws, r, total_col, f"=SUM(B{r}:M{r})",
                 bg=bg, fnt=fnt_style, aln=align("right"), fmt=MONEY_FMT)
        set_cell(ws, r, total_col+1, 0, bg=bg, fnt=font(color=INPUT_BLUE), aln=align("right"), fmt=MONEY_FMT)
        set_cell(ws, r, total_col+2, f"=N{r}-O{r}", bg=bg, fnt=fnt_style, aln=align("right"), fmt=MONEY_FMT)
        ws.row_dimensions[r].height = 16

    # Income total row
    annual_row(ws, r, "TOTAL INCOME", SAGE_LIGHT, font(bold=True, color=CHARCOAL),
               lambda mo: f"'{mo}'!C{rows_tracker[mo]['income_total']}")
    r += 1

    # Expense category subtotals
    for i, cat_name in enumerate(EXPENSE_CATS.keys()):
        cat_key = cat_name.lower().replace(" ","_").replace("&","").replace("__","_")
        bg = ROSE_PALE if i % 2 == 0 else WHITE
        annual_row(ws, r, cat_name, bg, font(color=CHARCOAL),
                   lambda mo, ck=cat_key: f"'{mo}'!C{rows_tracker[mo][f'{ck}_total']}")
        r += 1

    # Grand total
    annual_row(ws, r, "GRAND TOTAL EXPENSES", ROSE_DARK, font(bold=True, color=WHITE),
               lambda mo: f"'{mo}'!C{rows_tracker[mo]['grand_total']}")
    r += 1

    # Net cash flow
    annual_row(ws, r, "NET CASH FLOW", SAGE_DARK, font(bold=True, color=WHITE),
               lambda mo: f"'{mo}'!D{rows_tracker[mo]['net_flow']}")
    r += 1


# ── Dashboard ────────────────────────────────────────────────────────────────
def build_dashboard(wb, rows_tracker):
    ws = wb["Dashboard"]
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "A3"

    NCOLS = 14
    col_widths = [20,14,14,14,14,14,14,14,14,14,14,14,14,20]
    for i, w in enumerate(col_widths, 1):
        ws.column_dimensions[get_column_letter(i)].width = w

    # Row 1: Main title
    r = 1
    merge_row(ws, r, 1, NCOLS, "DASHBOARD · THE ULTIMATE MONTHLY BUDGET · 2026",
              ROSE_DARK, font(size=16, bold=True, color=WHITE))
    ws.row_dimensions[r].height = 35

    # Row 2: Subtitle
    r = 2
    merge_row(ws, r, 1, NCOLS, "Your complete financial overview at a glance",
              GOLD, font(size=10, italic=True, color=CHARCOAL))
    ws.row_dimensions[r].height = 22

    # Row 3: Section header
    r = 3
    merge_row(ws, r, 1, NCOLS, "BUDGET OVERVIEW",
              ROSE_MID, font(bold=True, color=WHITE))
    ws.row_dimensions[r].height = 22

    # Rows 4-9: KPI rows
    # Annual Income = sum of monthly income totals (actual, col D)
    inc_refs = "+".join(f"'{mo}'!D{rows_tracker[mo]['income_total']}" for mo in MONTHS)
    exp_refs = "+".join(f"'{mo}'!D{rows_tracker[mo]['grand_total']}" for mo in MONTHS)
    net_refs = "+".join(f"'{mo}'!D{rows_tracker[mo]['net_flow']}" for mo in MONTHS)

    # best/worst month by net flow
    net_range_d = ",".join(f"'{mo}'!D{rows_tracker[mo]['net_flow']}" for mo in MONTHS)

    kpis = [
        ("Annual Income",   f"={inc_refs}",   MONEY_FMT, SAGE_PALE),
        ("Annual Expenses",  f"={exp_refs}",   MONEY_FMT, ROSE_PALE),
        ("Annual Savings",   f"={net_refs}",   MONEY_FMT, SAGE_PALE),
        ("Savings Rate",     f"=IF(({inc_refs})=0,0,({net_refs})/({inc_refs}))", PCT_FMT, ROSE_PALE),
        ("Best Month",       f'=INDEX({{"Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"}},MATCH(MAX({net_range_d}),{{{net_range_d}}},0))', "@", SAGE_PALE),
        ("Worst Month",      f'=INDEX({{"Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"}},MATCH(MIN({net_range_d}),{{{net_range_d}}},0))', "@", ROSE_PALE),
    ]

    r = 4
    for label, formula, fmt, bg in kpis:
        set_cell(ws, r, 1, label, bg=bg, fnt=font(bold=True, color=CHARCOAL), aln=align())
        set_cell(ws, r, 2, formula, bg=bg, fnt=font(color=ROSE_DARK), aln=align("right"), fmt=fmt)
        for c in range(3, NCOLS+1):
            set_cell(ws, r, c, "", bg=bg)
        ws.row_dimensions[r].height = 18
        r += 1

    # ── Helper data for charts (rows 10-21, cols 16+) ─────────────────────────
    # Use col 16 (P) onward to avoid merge collisions
    # Row 10: labels
    HELPER_COL = 16  # col P

    # Monthly income/expenses/savings (actual col D)
    set_cell(ws, 10, HELPER_COL,   "Month")
    set_cell(ws, 10, HELPER_COL+1, "Monthly Income")
    set_cell(ws, 10, HELPER_COL+2, "Monthly Expenses")
    set_cell(ws, 10, HELPER_COL+3, "Monthly Savings")
    set_cell(ws, 10, HELPER_COL+4, "Cumulative Savings")

    cumulative_formula_parts = []
    for mi, mo_name in enumerate(MONTHS):
        rr = 11 + mi
        set_cell(ws, rr, HELPER_COL,   mo_name)
        set_cell(ws, rr, HELPER_COL+1,
                 f"='{mo_name}'!D{rows_tracker[mo_name]['income_total']}", fmt=MONEY_FMT)
        set_cell(ws, rr, HELPER_COL+2,
                 f"='{mo_name}'!D{rows_tracker[mo_name]['grand_total']}", fmt=MONEY_FMT)
        set_cell(ws, rr, HELPER_COL+3,
                 f"='{mo_name}'!D{rows_tracker[mo_name]['net_flow']}", fmt=MONEY_FMT)
        cumulative_formula_parts.append(f"{get_column_letter(HELPER_COL+3)}{rr}")
        set_cell(ws, rr, HELPER_COL+4,
                 f"=SUM({','.join(cumulative_formula_parts)})", fmt=MONEY_FMT)
        ws.row_dimensions[rr].height = 16

    # Income by source (for doughnut) — rows 24-30
    set_cell(ws, 24, HELPER_COL,   "Income Source")
    set_cell(ws, 24, HELPER_COL+1, "Annual Amount")
    for i, src in enumerate(INCOME_SOURCES):
        rr = 25 + i
        set_cell(ws, rr, HELPER_COL, src)
        # sum across Jan..Dec for this income row
        mo_refs = "+".join(
            f"'Jan'!D{rows_tracker['Jan']['income_start']+i}"
            if mo == 'Jan'
            else f"'{mo}'!D{rows_tracker[mo]['income_start']+i}"
            for mo in MONTHS
        )
        set_cell(ws, rr, HELPER_COL+1, f"={mo_refs}", fmt=MONEY_FMT)

    # ── Charts ────────────────────────────────────────────────────────────────
    # Chart 1: BarChart - Annual Income by Month
    chart1 = BarChart()
    chart1.type = "col"
    chart1.title = "Monthly Income"
    chart1.y_axis.title = "Amount"
    chart1.x_axis.title = "Month"
    chart1.width = 14
    chart1.height = 10
    chart1.style = 10

    data1 = Reference(ws, min_col=HELPER_COL+1, min_row=10, max_row=22)
    cats1 = Reference(ws, min_col=HELPER_COL,   min_row=11, max_row=22)
    chart1.add_data(data1, titles_from_data=True)
    chart1.set_categories(cats1)
    if chart1.series:
        chart1.series[0].graphicalProperties.solidFill = CHART_ROSE
        chart1.series[0].graphicalProperties.line.solidFill = CHART_ROSE
    ws.add_chart(chart1, "A22")

    # Chart 2: DoughnutChart - Annual Income by Source
    chart2 = DoughnutChart()
    chart2.title = "Annual Income by Source"
    chart2.width = 14
    chart2.height = 10
    chart2.style = 10
    chart2.holeSize = 40

    data2 = Reference(ws, min_col=HELPER_COL+1, min_row=24, max_row=30)
    cats2 = Reference(ws, min_col=HELPER_COL,   min_row=25, max_row=30)
    chart2.add_data(data2, titles_from_data=True)
    chart2.set_categories(cats2)
    ws.add_chart(chart2, "H22")

    # Chart 3: BarChart horizontal - Annual Savings by Savings Name
    st_ws = wb["Savings Tracker"]
    chart3 = BarChart()
    chart3.type = "bar"  # horizontal
    chart3.title = "Savings Goals Progress"
    chart3.y_axis.title = "Goal"
    chart3.x_axis.title = "Amount"
    chart3.width = 14
    chart3.height = 10
    chart3.style = 10

    data3 = Reference(st_ws, min_col=3, min_row=3, max_row=11)
    cats3 = Reference(st_ws, min_col=1, min_row=4, max_row=11)
    chart3.add_data(data3, titles_from_data=True)
    chart3.set_categories(cats3)
    if chart3.series:
        chart3.series[0].graphicalProperties.solidFill = CHART_GREEN
        chart3.series[0].graphicalProperties.line.solidFill = CHART_GREEN
    ws.add_chart(chart3, "A36")

    # Chart 4: BarChart - Monthly Savings
    chart4 = BarChart()
    chart4.type = "col"
    chart4.title = "Monthly Savings"
    chart4.y_axis.title = "Amount"
    chart4.x_axis.title = "Month"
    chart4.width = 14
    chart4.height = 10
    chart4.style = 10

    data4 = Reference(ws, min_col=HELPER_COL+3, min_row=10, max_row=22)
    cats4 = Reference(ws, min_col=HELPER_COL,   min_row=11, max_row=22)
    chart4.add_data(data4, titles_from_data=True)
    chart4.set_categories(cats4)
    if chart4.series:
        chart4.series[0].graphicalProperties.solidFill = CHART_GREEN
        chart4.series[0].graphicalProperties.line.solidFill = CHART_GREEN
    ws.add_chart(chart4, "H36")


# ── Main ──────────────────────────────────────────────────────────────────────
def main():
    wb = Workbook()
    # Remove default sheet
    wb.remove(wb.active)

    # Create sheets in order
    sheet_names = (
        ["Dashboard"]
        + MONTHS
        + ["Annual Budget", "Savings Tracker"]
    )
    for name in sheet_names:
        wb.create_sheet(name)

    # Set Calibri as default font
    from openpyxl.styles.named_styles import NamedStyle
    normal = NamedStyle(name="normal_calibri")
    normal.font = Font(name="Calibri", size=11)
    wb.add_named_style(normal)

    rows = {}

    # Build monthly sheets first (dashboard needs rows_tracker)
    for mo_name in MONTHS:
        build_monthly(wb, mo_name, rows)

    # Build other sheets
    build_savings_tracker(wb, rows)
    build_annual(wb, rows)
    build_dashboard(wb, rows)

    out_path = "/home/user/ouroboros/ultimate_monthly_budget.xlsx"
    wb.save(out_path)
    print(f"Saved: {out_path}")


if __name__ == "__main__":
    main()
