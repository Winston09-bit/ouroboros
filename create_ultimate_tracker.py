"""
Premium Budget Tracker — ultimate_budget_tracker.xlsx
Etsy-style: sage green + terracotta, no gridlines, Calibri, clean layout.
"""

from openpyxl import Workbook
from openpyxl.styles import (
    Font, PatternFill, Alignment, Border, Side, numbers
)
from openpyxl.utils import get_column_letter
from openpyxl.worksheet.datavalidation import DataValidation
from openpyxl.formatting.rule import CellIsRule, FormulaRule

wb = Workbook()

# ─── PALETTE ────────────────────────────────────────────────────────────────
SAGE_DARK   = "3D5C3A"
SAGE_MID    = "6B8F5E"
SAGE_LIGHT  = "C8DDB8"
SAGE_PALE   = "EEF4E8"
TERRA       = "B05E3A"
TERRA_LIGHT = "EDD5C5"
CREAM       = "FAF7F2"
GOLD        = "BFA06A"
WHITE       = "FFFFFF"
CHARCOAL    = "2A2A2A"
INPUT_BLUE  = "1A4FA0"
FORMULA_BLK = "2A2A2A"
LINK_GREEN  = "1A6B2A"
RED_ALERT   = "B03A2E"
GREEN_OK    = "1E8449"
BORDER_GRAY = "DCDCDC"

FMT_CURR = '#,##0;[RED]-#,##0;-'
FMT_ZERO = '#,##0;[RED]-#,##0;-'
FMT_PCT  = '0.0%;[RED]-0.0%'
FMT_DATE = 'MM/DD/YYYY'

MONTHS = ["Jan","Feb","Mar","Apr","May","Jun",
          "Jul","Aug","Sep","Oct","Nov","Dec"]
MONTH_NAMES = ["January","February","March","April","May","June",
               "July","August","September","October","November","December"]

# ─── STYLE HELPERS ──────────────────────────────────────────────────────────
def fl(hex_color):
    return PatternFill("solid", fgColor=hex_color)

def fn(size=9, bold=False, color=CHARCOAL, italic=False):
    return Font(name="Calibri", size=size, bold=bold, color=color, italic=italic)

def al(h="left", v="center", wrap=False):
    return Alignment(horizontal=h, vertical=v, wrap_text=wrap)

THIN_GRAY = Side(style="thin", color=BORDER_GRAY)
MED_SAGE  = Side(style="medium", color=SAGE_MID)
NO_BORDER = Side(style=None)

def thin_border():
    return Border(left=THIN_GRAY, right=THIN_GRAY,
                  top=THIN_GRAY, bottom=THIN_GRAY)

def section_border():
    return Border(left=MED_SAGE, right=MED_SAGE,
                  top=MED_SAGE, bottom=MED_SAGE)

def bottom_med():
    return Border(left=THIN_GRAY, right=THIN_GRAY,
                  top=THIN_GRAY, bottom=Side(style="medium", color=SAGE_MID))

# ─── BASE CELL WRITERS ──────────────────────────────────────────────────────
def title_cell(ws, row, col1, col2, text, subtitle=False):
    ws.merge_cells(start_row=row, start_column=col1,
                   end_row=row, end_column=col2)
    c = ws.cell(row=row, column=col1, value=text)
    if subtitle:
        c.font      = fn(10, False, WHITE, italic=True)
        c.fill      = fl(GOLD)
        c.alignment = al("left")
        ws.row_dimensions[row].height = 18
    else:
        c.font      = fn(18, True, WHITE)
        c.fill      = fl(SAGE_DARK)
        c.alignment = al("center")
        ws.row_dimensions[row].height = 48
    c.border = thin_border()
    return c

def col_header(ws, row, col, text, bg=SAGE_DARK):
    c = ws.cell(row=row, column=col, value=text)
    c.font      = fn(9, True, WHITE)
    c.fill      = fl(bg)
    c.alignment = al("center")
    c.border    = thin_border()
    ws.row_dimensions[row].height = 20
    return c

def section_hdr(ws, row, col1, col2, text, bg=SAGE_MID):
    ws.merge_cells(start_row=row, start_column=col1,
                   end_row=row, end_column=col2)
    c = ws.cell(row=row, column=col1, value=text)
    c.font      = fn(10, True, WHITE)
    c.fill      = fl(bg)
    c.alignment = al("left")
    c.border    = thin_border()
    ws.row_dimensions[row].height = 22
    return c

def lbl(ws, row, col, text, bold=False, bg=None, color=CHARCOAL):
    c = ws.cell(row=row, column=col, value=text)
    c.font      = fn(9, bold, color)
    c.fill      = fl(bg) if bg else fl(WHITE)
    c.alignment = al("left")
    c.border    = thin_border()
    ws.row_dimensions[row].height = 18
    return c

def inp(ws, row, col, value=0, fmt=FMT_CURR, bg=WHITE):
    c = ws.cell(row=row, column=col, value=value)
    c.font           = fn(9, False, INPUT_BLUE)
    c.fill           = fl(bg)
    c.alignment      = al("right")
    c.border         = thin_border()
    c.number_format  = fmt
    ws.row_dimensions[row].height = 18
    return c

def frm(ws, row, col, formula, fmt=FMT_CURR, color=FORMULA_BLK, bg=WHITE):
    c = ws.cell(row=row, column=col, value=formula)
    c.font           = fn(9, False, color)
    c.fill           = fl(bg)
    c.alignment      = al("right")
    c.border         = thin_border()
    c.number_format  = fmt
    ws.row_dimensions[row].height = 18
    return c

def subtotal_row(ws, row, col1, col2, label, sum_col_refs, bg=SAGE_LIGHT):
    if col1 + 1 <= col2:
        ws.merge_cells(start_row=row, start_column=col1,
                       end_row=row, end_column=col1+1)
    c = ws.cell(row=row, column=col1, value=label)
    c.font      = fn(9, True, CHARCOAL)
    c.fill      = fl(bg)
    c.alignment = al("left")
    c.border    = bottom_med()
    ws.row_dimensions[row].height = 20
    for col, formula in sum_col_refs:
        # skip if cell is part of a merge
        cell_obj = ws.cell(row=row, column=col)
        if hasattr(cell_obj, 'value'):
            try:
                cell_obj.value = formula
                cell_obj.font          = fn(9, True, CHARCOAL)
                cell_obj.fill          = fl(bg)
                cell_obj.alignment     = al("right")
                cell_obj.border        = bottom_med()
                cell_obj.number_format = FMT_CURR
            except AttributeError:
                pass

def grand_total_row(ws, row, col1, col2, label, refs, bg=TERRA):
    ws.merge_cells(start_row=row, start_column=col1,
                   end_row=row, end_column=col1+1)
    c = ws.cell(row=row, column=col1, value=label)
    c.font      = fn(10, True, WHITE)
    c.fill      = fl(bg)
    c.alignment = al("left")
    c.border    = thin_border()
    ws.row_dimensions[row].height = 22
    for col, formula in refs:
        fc = ws.cell(row=row, column=col, value=formula)
        fc.font          = fn(10, True, WHITE)
        fc.fill          = fl(bg)
        fc.alignment     = al("right")
        fc.border        = thin_border()
        fc.number_format = FMT_CURR

def net_flow_row(ws, row, label, formula_b, formula_d, bg=SAGE_DARK):
    ws.merge_cells(start_row=row, start_column=1,
                   end_row=row, end_column=2)
    c = ws.cell(row=row, column=1, value=label)
    c.font      = fn(10, True, WHITE)
    c.fill      = fl(bg)
    c.alignment = al("left")
    c.border    = thin_border()
    ws.row_dimensions[row].height = 22
    for col, formula in [(3, formula_b), (4, formula_d)]:
        fc = ws.cell(row=row, column=col, value=formula)
        fc.font          = fn(10, True, WHITE)
        fc.fill          = fl(bg)
        fc.alignment     = al("right")
        fc.border        = thin_border()
        fc.number_format = FMT_CURR
    # Empty cols 5-7
    for col in [5, 6, 7]:
        ec = ws.cell(row=row, column=col)
        ec.fill   = fl(bg)
        ec.border = thin_border()

def fill_row_bg(ws, row, num_cols, bg, start_col=1):
    for c in range(start_col, start_col + num_cols):
        cell = ws.cell(row=row, column=c)
        if cell.fill.fgColor.rgb in ("00000000", "FFFFFFFF", ""):
            cell.fill = fl(bg)

def freeze(ws, cell="A3"):
    ws.freeze_panes = cell

def no_gridlines(ws):
    ws.sheet_view.showGridLines = False

def set_widths(ws, widths):
    for col, w in widths.items():
        ws.column_dimensions[col].width = w

def cream_background(ws, max_row=200, max_col=20):
    """Fill all cells cream to remove default gray."""
    for r in range(1, max_row + 1):
        for c in range(1, max_col + 1):
            cell = ws.cell(row=r, column=c)
            if cell.fill.patternType is None or cell.fill.fgColor.rgb in ("00000000",):
                cell.fill = fl(CREAM)

# ═══════════════════════════════════════════════════════════════════════════
# MONTHLY SHEET BUILDER — returns dict of key row numbers
# ═══════════════════════════════════════════════════════════════════════════
EXPENSE_CATS = [
    ("HOUSING",                ["Rent/Mortgage","Utilities","Electricity","Internet","Home Insurance"]),
    ("TRANSPORT",              ["Car Payment","Fuel","Car Insurance","Parking","Public Transport"]),
    ("FOOD & DINING",          ["Groceries","Restaurants","Coffee","Food Delivery","Work Lunch"]),
    ("HEALTH & WELLNESS",      ["Health Insurance","Gym","Pharmacy","Doctor","Dental"]),
    ("LIFESTYLE",              ["Streaming Services","Entertainment","Hobbies","Personal Care","Books"]),
    ("PERSONAL",               ["Clothing","Gifts","Education","Pet Care","Miscellaneous"]),
    ("SAVINGS & INVESTMENTS",  ["Emergency Fund","Pension/IRA","Stocks","Vacation Fund","Other Savings"]),
    ("DEBT PAYMENTS",          ["Credit Card 1","Credit Card 2","Student Loan","Car Loan","Other"]),
]

MONTHLY_HEADERS = ["CATEGORY","SUBCATEGORY","BUDGETED","ACTUAL","DIFFERENCE","% USED","NOTES"]

def build_month_sheet(ws, month_name, month_short):
    no_gridlines(ws)

    # Row 1: title, Row 2: subtitle
    max_col = 7
    title_cell(ws, 1, 1, max_col, f"{month_name.upper()} · MONTHLY BUDGET")
    title_cell(ws, 2, 1, max_col,
               f"Track your income and expenses for {month_name}", subtitle=True)

    # Row 3: column headers
    for i, h in enumerate(MONTHLY_HEADERS):
        col_header(ws, 3, i+1, h)

    r = 4  # current row pointer

    # ── INCOME section ──────────────────────────────────────────────
    section_hdr(ws, r, 1, max_col, "INCOME", bg=TERRA)
    r += 1
    income_start = r
    income_items = ["Primary Income","Side Income","Other Income"]
    for j, item in enumerate(income_items):
        bg = TERRA_LIGHT if j % 2 else CREAM
        lbl(ws, r, 1, "INCOME", bg=bg)
        lbl(ws, r, 2, item, bg=bg)
        inp(ws, r, 3, bg=bg)
        inp(ws, r, 4, bg=bg)
        frm(ws, r, 5, f"=IFERROR(C{r}-D{r},0)", bg=bg)
        frm(ws, r, 6, f"=IFERROR(D{r}/C{r},0)", FMT_PCT, bg=bg)
        lbl(ws, r, 7, "", bg=bg)
        r += 1
    income_end = r - 1

    # Total Income
    ti_row = r
    subtotal_row(ws, ti_row, 1, max_col, "TOTAL INCOME", [
        (3, f"=SUM(C{income_start}:C{income_end})"),
        (4, f"=SUM(D{income_start}:D{income_end})"),
        (5, f"=IFERROR(C{ti_row}-D{ti_row},0)"),
    ], bg=SAGE_LIGHT)
    ws.cell(row=ti_row, column=6).value = f"=IFERROR(D{ti_row}/C{ti_row},0)"
    ws.cell(row=ti_row, column=6).number_format = FMT_PCT
    ws.cell(row=ti_row, column=6).fill = fl(SAGE_LIGHT)
    ws.cell(row=ti_row, column=6).font = fn(9, True)
    ws.cell(row=ti_row, column=6).border = bottom_med()
    r += 1

    # ── EXPENSE sections ─────────────────────────────────────────────
    subtotal_rows = {}
    for cat_name, sub_items in EXPENSE_CATS:
        section_hdr(ws, r, 1, max_col, cat_name, bg=SAGE_MID)
        r += 1
        sub_start = r
        for j, sub in enumerate(sub_items):
            bg = SAGE_PALE if j % 2 else CREAM
            lbl(ws, r, 1, cat_name, bg=bg)
            lbl(ws, r, 2, sub, bg=bg)
            inp(ws, r, 3, bg=bg)
            inp(ws, r, 4, bg=bg)
            frm(ws, r, 5, f"=IFERROR(C{r}-D{r},0)", bg=bg)
            frm(ws, r, 6, f"=IFERROR(D{r}/C{r},0)", FMT_PCT, bg=bg)
            lbl(ws, r, 7, "", bg=bg)
            r += 1
        sub_end = r - 1
        sub_r = r
        subtotal_row(ws, sub_r, 1, max_col, f"{cat_name} TOTAL", [
            (3, f"=SUM(C{sub_start}:C{sub_end})"),
            (4, f"=SUM(D{sub_start}:D{sub_end})"),
            (5, f"=IFERROR(C{sub_r}-D{sub_r},0)"),
        ])
        ws.cell(row=sub_r, column=6).value = f"=IFERROR(D{sub_r}/C{sub_r},0)"
        ws.cell(row=sub_r, column=6).number_format = FMT_PCT
        ws.cell(row=sub_r, column=6).fill = fl(SAGE_LIGHT)
        ws.cell(row=sub_r, column=6).font = fn(9, True)
        ws.cell(row=sub_r, column=6).border = bottom_med()
        subtotal_rows[cat_name] = sub_r
        r += 1

    # GRAND TOTAL EXPENSES
    gt_row = r
    sub_refs_b = "+".join([f"C{subtotal_rows[c]}" for c, _ in EXPENSE_CATS])
    sub_refs_a = "+".join([f"D{subtotal_rows[c]}" for c, _ in EXPENSE_CATS])
    grand_total_row(ws, gt_row, 1, max_col, "GRAND TOTAL EXPENSES", [
        (3, f"={sub_refs_b}"),
        (4, f"={sub_refs_a}"),
        (5, f"=IFERROR(C{gt_row}-D{gt_row},0)"),
    ])
    r += 1

    # NET CASH FLOW
    ncf_row = r
    net_flow_row(ws, ncf_row, "NET CASH FLOW",
                 f"=IFERROR(C{ti_row}-C{gt_row},0)",
                 f"=IFERROR(D{ti_row}-D{gt_row},0)")
    r += 1

    freeze(ws, "A3")
    no_gridlines(ws)
    ws.sheet_properties.tabColor = SAGE_MID
    set_widths(ws, {"A":26,"B":24,"C":14,"D":14,"E":14,"F":10,"G":20})

    return {
        "ti_row": ti_row,
        "gt_row": gt_row,
        "ncf_row": ncf_row,
        "subtotals": subtotal_rows,
    }

# ═══════════════════════════════════════════════════════════════════════════
# BUILD ALL SHEETS
# ═══════════════════════════════════════════════════════════════════════════

# Remove default sheet
ws_dash = wb.active
ws_dash.title = "Dashboard"

# Build 12 monthly sheets first (needed for Dashboard formula refs)
month_meta = {}
for m_short, m_long in zip(MONTHS, MONTH_NAMES):
    ws_m = wb.create_sheet(title=m_short)
    meta = build_month_sheet(ws_m, m_long, m_short)
    month_meta[m_short] = meta

# ═══════════════════════════════════════════════════════════════════════════
# 1. DASHBOARD
# ═══════════════════════════════════════════════════════════════════════════
no_gridlines(ws_dash)

title_cell(ws_dash, 1, 1, 9, "PERSONAL FINANCE COMMAND CENTER · 2026")
title_cell(ws_dash, 2, 1, 9, "Live Summary Dashboard", subtitle=True)

freeze(ws_dash, "A3")
ws_dash.sheet_properties.tabColor = SAGE_DARK

# ── LEFT PANEL: BUDGET SUMMARY ────────────────────────────────────────────
jan_meta = month_meta["Jan"]
jan_gt   = jan_meta["gt_row"]
jan_ti   = jan_meta["ti_row"]

section_hdr(ws_dash, 3, 1, 4, "BUDGET SUMMARY")
bs_headers = ["METRIC","","VALUE",""]
for i, h in enumerate(bs_headers):
    col_header(ws_dash, 4, i+1, h)

budget_items = [
    ("Annual Income",        True,  None,        "D"),
    ("Monthly Income",       False, "=IFERROR(D5/12,0)", "D"),
    ("This Month Expenses",  False, f"=IFERROR(Jan!D{jan_gt},0)", "D"),
    ("Left to Spend",        False, "=IFERROR(D6-D7,0)", "D"),
    ("Annual Savings Rate",  False, "=IFERROR(D5/12-D7,0)/IFERROR(D6,1)", "D"),
    ("YTD Total Savings",    False, None,         "D"),
]

# Build YTD savings formula from all months savings subtotal
sav_cat = "SAVINGS & INVESTMENTS"
ytd_parts = [f"{m}!D{month_meta[m]['subtotals'][sav_cat]}" for m in MONTHS]
ytd_formula = "=IFERROR(" + "+".join(ytd_parts) + ",0)"

actual_formulas = [None, "=IFERROR(D5/12,0)",
                   f"=IFERROR(Jan!D{jan_gt},0)",
                   "=IFERROR(D6-D7,0)",
                   "=IFERROR(IFERROR(D6-D7,0)/IFERROR(D6,1),0)",
                   ytd_formula]
actual_is_input = [True, False, False, False, False, False]
actual_fmts     = [FMT_CURR, FMT_CURR, FMT_CURR, FMT_CURR, FMT_PCT, FMT_CURR]
actual_colors   = [INPUT_BLUE, LINK_GREEN, LINK_GREEN, FORMULA_BLK, FORMULA_BLK, LINK_GREEN]
actual_labels   = ["Annual Income","Monthly Income","This Month Expenses",
                   "Left to Spend","Annual Savings Rate","YTD Total Savings"]

for i, (lbl_text, formula, is_inp, fmt, color) in enumerate(zip(
        actual_labels, actual_formulas, actual_is_input, actual_fmts, actual_colors)):
    r = 5 + i
    bg = SAGE_PALE if i % 2 else CREAM
    ws_dash.merge_cells(start_row=r, start_column=1, end_row=r, end_column=2)
    lbl(ws_dash, r, 1, lbl_text, bold=True, bg=bg)
    ws_dash.cell(row=r, column=2).fill = fl(bg)
    ws_dash.cell(row=r, column=2).border = thin_border()
    ws_dash.merge_cells(start_row=r, start_column=3, end_row=r, end_column=4)
    if is_inp:
        inp(ws_dash, r, 3, 0, fmt, bg)
    else:
        frm(ws_dash, r, 3, formula, fmt, color, bg)
    ws_dash.cell(row=r, column=4).fill = fl(bg)
    ws_dash.cell(row=r, column=4).border = thin_border()
    ws_dash.row_dimensions[r].height = 18

# ── RIGHT PANEL: SAVINGS SNAPSHOT ────────────────────────────────────────
section_hdr(ws_dash, 3, 6, 9, "SAVINGS SNAPSHOT", bg=TERRA)
snap_headers = ["GOAL","","AMOUNT",""]
for i, h in enumerate(snap_headers):
    col_header(ws_dash, 4, 6+i, h, bg=TERRA)

snap_labels  = ["Emergency Fund Goal","Emergency Fund Current","% Funded",
                "Monthly Target","YTD Saved"]
snap_inp     = [True, True, False, True, False]
snap_fmts    = [FMT_CURR, FMT_CURR, FMT_PCT, FMT_CURR, FMT_CURR]
snap_formulas= [None, None, "=IFERROR(H6/H5,0)", None, ytd_formula]
snap_colors  = [INPUT_BLUE, INPUT_BLUE, FORMULA_BLK, INPUT_BLUE, LINK_GREEN]

for i, (sl, si, sf_fmt, sf, sc) in enumerate(zip(
        snap_labels, snap_inp, snap_fmts, snap_formulas, snap_colors)):
    r = 5 + i
    bg = TERRA_LIGHT if i % 2 else CREAM
    ws_dash.merge_cells(start_row=r, start_column=6, end_row=r, end_column=7)
    lbl(ws_dash, r, 6, sl, bold=True, bg=bg)
    ws_dash.cell(row=r, column=7).fill = fl(bg)
    ws_dash.cell(row=r, column=7).border = thin_border()
    ws_dash.merge_cells(start_row=r, start_column=8, end_row=r, end_column=9)
    if si:
        inp(ws_dash, r, 8, 0, sf_fmt, bg)
    else:
        frm(ws_dash, r, 8, sf, sf_fmt, sc, bg)
    ws_dash.cell(row=r, column=9).fill = fl(bg)
    ws_dash.cell(row=r, column=9).border = thin_border()

# ── MID LEFT: TOP 6 EXPENSE CATEGORIES ───────────────────────────────────
section_hdr(ws_dash, 15, 1, 4, "TOP EXPENSE CATEGORIES — JANUARY")
for i, h in enumerate(["CATEGORY","BUDGETED","ACTUAL",""]):
    col_header(ws_dash, 16, i+1, h)

cat_names = [c for c, _ in EXPENSE_CATS[:6]]
for i, cat in enumerate(cat_names):
    r = 17 + i
    bg = SAGE_PALE if i % 2 else CREAM
    sub_r = month_meta["Jan"]["subtotals"][cat]
    lbl(ws_dash, r, 1, cat, bg=bg)
    frm(ws_dash, r, 2, f"=IFERROR(Jan!C{sub_r},0)", color=LINK_GREEN, bg=bg)
    frm(ws_dash, r, 3, f"=IFERROR(Jan!D{sub_r},0)", color=LINK_GREEN, bg=bg)
    lbl(ws_dash, r, 4, "", bg=bg)

# ── MID RIGHT: BILLS THIS MONTH ───────────────────────────────────────────
section_hdr(ws_dash, 15, 6, 9, "BILLS THIS MONTH", bg=TERRA)
for i, h in enumerate(["BILL NAME","AMOUNT","DUE DATE",""]):
    col_header(ws_dash, 16, 6+i, h, bg=TERRA)

sample_bills = [
    ("Rent/Mortgage",  1200, "01/01/2026"),
    ("Electricity",      80, "01/15/2026"),
    ("Internet",         60, "01/22/2026"),
    ("Streaming",        18, "01/08/2026"),
    ("Phone",            70, "01/12/2026"),
    ("Car Insurance",   120, "01/20/2026"),
]
for i, (bname, bamt, bdate) in enumerate(sample_bills):
    r = 17 + i
    bg = TERRA_LIGHT if i % 2 else CREAM
    lbl(ws_dash, r, 6, bname, bg=bg)
    inp(ws_dash, r, 7, bamt, FMT_CURR, bg)
    c_date = ws_dash.cell(row=r, column=8, value=bdate)
    c_date.font = fn(9, False, INPUT_BLUE)
    c_date.fill = fl(bg)
    c_date.alignment = al("right")
    c_date.border = thin_border()
    c_date.number_format = FMT_DATE
    lbl(ws_dash, r, 9, "", bg=bg)

# Chart placeholder
r_chart = 25
ws_dash.merge_cells(f"A{r_chart}:I{r_chart}")
cp = ws_dash.cell(row=r_chart, column=1,
    value="INSERT CHART — Annual Income vs Expenses (from Annual Overview)")
cp.font      = fn(9, True, SAGE_MID, italic=True)
cp.fill      = fl(SAGE_PALE)
cp.alignment = al("center")
cp.border    = section_border()
ws_dash.row_dimensions[r_chart].height = 30

set_widths(ws_dash, {"A":26,"B":14,"C":14,"D":14,"E":3,"F":26,"G":14,"H":14,"I":14})


# ═══════════════════════════════════════════════════════════════════════════
# 3. DEBT TRACKER
# ═══════════════════════════════════════════════════════════════════════════
ws_debt = wb.create_sheet("Debt Tracker")
no_gridlines(ws_debt)
ws_debt.sheet_properties.tabColor = TERRA

debt_cols = ["DEBT NAME","LENDER","ORIGINAL BAL","CURRENT BAL",
             "INTEREST %","MIN PAYMENT","MONTHLY PMT","EST PAYOFF DATE","STATUS"]
title_cell(ws_debt, 1, 1, len(debt_cols), "DEBT PAYOFF TRACKER")
title_cell(ws_debt, 2, 1, len(debt_cols),
           "Track balances, rates, and payoff progress", subtitle=True)
for i, h in enumerate(debt_cols):
    col_header(ws_debt, 3, i+1, h)

debts = ["Credit Card 1","Credit Card 2","Student Loan",
         "Car Loan","Personal Loan","Other Debt"]
for j, debt in enumerate(debts):
    r = 4 + j
    bg = TERRA_LIGHT if j % 2 else CREAM
    lbl(ws_debt, r, 1, debt, bg=bg)
    inp(ws_debt, r, 2, "", fmt="@", bg=bg)
    ws_debt.cell(row=r, column=2).number_format = "@"
    inp(ws_debt, r, 3, 0, FMT_CURR, bg)
    inp(ws_debt, r, 4, 0, FMT_CURR, bg)
    inp(ws_debt, r, 5, 0, FMT_PCT, bg)
    inp(ws_debt, r, 6, 0, FMT_CURR, bg)
    inp(ws_debt, r, 7, 0, FMT_CURR, bg)
    inp(ws_debt, r, 8, "", FMT_DATE, bg)
    lbl(ws_debt, r, 9, "Active", bg=bg)

# Totals
tot_r = 4 + len(debts)
grand_total_row(ws_debt, tot_r, 1, 9, "TOTALS", [
    (3, f"=SUM(C4:C{tot_r-1})"),
    (4, f"=SUM(D4:D{tot_r-1})"),
    (6, f"=SUM(F4:F{tot_r-1})"),
    (7, f"=SUM(G4:G{tot_r-1})"),
])

# Summary box
sb = tot_r + 2
section_hdr(ws_debt, sb, 1, 5, "DEBT SUMMARY")
summary_data = [
    ("Total Outstanding Debt",    f"=IFERROR(SUM(D4:D{tot_r-1}),0)", FMT_CURR),
    ("Total Monthly Minimums",    f"=IFERROR(SUM(F4:F{tot_r-1}),0)", FMT_CURR),
    ("Debt-to-Income Ratio",      f"=IFERROR(SUM(D4:D{tot_r-1})/(Dashboard!D5/12),0)", FMT_PCT),
    ("Highest Interest Rate Debt",f'=IFERROR(INDEX(A4:A{tot_r-1},MATCH(MAX(E4:E{tot_r-1}),E4:E{tot_r-1},0)),"N/A")', "@"),
]
for k, (sl, sf, sfmt) in enumerate(summary_data):
    r = sb + 1 + k
    bg = SAGE_PALE if k % 2 else CREAM
    ws_debt.merge_cells(start_row=r, start_column=1, end_row=r, end_column=3)
    lbl(ws_debt, r, 1, sl, bold=True, bg=bg)
    ws_debt.cell(row=r, column=2).fill = fl(bg)
    ws_debt.cell(row=r, column=2).border = thin_border()
    ws_debt.cell(row=r, column=3).fill = fl(bg)
    ws_debt.cell(row=r, column=3).border = thin_border()
    ws_debt.merge_cells(start_row=r, start_column=4, end_row=r, end_column=5)
    frm(ws_debt, r, 4, sf, sfmt, LINK_GREEN, bg)
    ws_debt.cell(row=r, column=5).fill = fl(bg)
    ws_debt.cell(row=r, column=5).border = thin_border()

freeze(ws_debt, "A3")
set_widths(ws_debt, {"A":22,"B":18,"C":14,"D":14,"E":12,"F":14,"G":14,"H":14,"I":12})


# ═══════════════════════════════════════════════════════════════════════════
# 4. BILL CALENDAR
# ═══════════════════════════════════════════════════════════════════════════
ws_bill = wb.create_sheet("Bill Calendar")
no_gridlines(ws_bill)
ws_bill.sheet_properties.tabColor = TERRA

bill_cols = ["BILL NAME","CATEGORY","AMOUNT","DUE DATE",
             "FREQUENCY","AUTO-PAY","PAID?","ANNUAL COST","NOTES"]
title_cell(ws_bill, 1, 1, len(bill_cols), "BILL CALENDAR · 2026")
title_cell(ws_bill, 2, 1, len(bill_cols),
           "Track recurring bills, auto-payments, and annual costs", subtitle=True)
for i, h in enumerate(bill_cols):
    col_header(ws_bill, 3, i+1, h)

bills_data = [
    ("Rent/Mortgage",    "Housing",       1200, "01/01/2026", "Monthly",  "No",  "No"),
    ("Electricity",      "Utilities",       80, "01/15/2026", "Monthly",  "Yes", "No"),
    ("Internet",         "Utilities",       60, "01/22/2026", "Monthly",  "Yes", "No"),
    ("Phone",            "Subscriptions",   70, "01/12/2026", "Monthly",  "Yes", "No"),
    ("Car Insurance",    "Insurance",      120, "01/20/2026", "Monthly",  "No",  "No"),
    ("Streaming",        "Subscriptions",   18, "01/08/2026", "Monthly",  "Yes", "No"),
    ("Gym Membership",   "Health",          45, "01/01/2026", "Monthly",  "Yes", "No"),
    ("Spotify",          "Subscriptions",   11, "01/05/2026", "Monthly",  "Yes", "No"),
    ("Home Insurance",   "Insurance",      150, "01/01/2026", "Annual",   "No",  "No"),
    ("Car Registration", "Transport",       85, "03/01/2026", "Annual",   "No",  "No"),
]
for j, (bn, bc, ba, bd, bf, bap, bpd) in enumerate(bills_data):
    r = 4 + j
    bg = TERRA_LIGHT if j % 2 else CREAM
    lbl(ws_bill, r, 1, bn,  bg=bg)
    lbl(ws_bill, r, 2, bc,  bg=bg)
    inp(ws_bill, r, 3, ba,  FMT_CURR, bg)
    cd = ws_bill.cell(row=r, column=4, value=bd)
    cd.font = fn(9, False, INPUT_BLUE)
    cd.fill = fl(bg)
    cd.alignment = al("right")
    cd.border = thin_border()
    cd.number_format = FMT_DATE
    lbl(ws_bill, r, 5, bf,  bg=bg)
    lbl(ws_bill, r, 6, bap, bg=bg)
    lbl(ws_bill, r, 7, bpd, bg=bg)
    # Annual Cost formula
    ac = ws_bill.cell(row=r, column=8,
        value=f'=IFERROR(IF(E{r}="Monthly",C{r}*12,IF(E{r}="Annual",C{r},IF(E{r}="Quarterly",C{r}*4,C{r}*52))),0)')
    ac.font = fn(9, False, FORMULA_BLK)
    ac.fill = fl(bg)
    ac.alignment = al("right")
    ac.border = thin_border()
    ac.number_format = FMT_CURR
    lbl(ws_bill, r, 9, "", bg=bg)

bill_end = 4 + len(bills_data) - 1
tot_r_bill = bill_end + 1
grand_total_row(ws_bill, tot_r_bill, 1, len(bill_cols), "MONTHLY TOTAL", [
    (3, f"=SUM(C4:C{bill_end})"),
    (8, f"=SUM(H4:H{bill_end})"),
])

# Data validation
dv_cat  = DataValidation(type="list",
    formula1='"Housing,Utilities,Insurance,Subscriptions,Health,Transport,Other"',
    allow_blank=True)
dv_freq = DataValidation(type="list",
    formula1='"Monthly,Weekly,Bi-Weekly,Quarterly,Annual"', allow_blank=True)
dv_yn   = DataValidation(type="list", formula1='"Yes,No"', allow_blank=True)
ws_bill.add_data_validation(dv_cat);  dv_cat.sqref  = f"B4:B{bill_end}"
ws_bill.add_data_validation(dv_freq); dv_freq.sqref = f"E4:E{bill_end}"
ws_bill.add_data_validation(dv_yn);   dv_yn.sqref   = f"F4:G{bill_end}"

freeze(ws_bill, "A3")
set_widths(ws_bill, {"A":22,"B":16,"C":12,"D":12,"E":12,"F":10,"G":8,"H":14,"I":22})


# ═══════════════════════════════════════════════════════════════════════════
# 5. SAVINGS GOALS
# ═══════════════════════════════════════════════════════════════════════════
ws_sav = wb.create_sheet("Savings Goals")
no_gridlines(ws_sav)
ws_sav.sheet_properties.tabColor = SAGE_DARK

sav_cols = ["GOAL","TARGET AMOUNT","SAVED SO FAR","MONTHLY CONTRIB",
            "TARGET DATE","% COMPLETE","PROGRESS","PRIORITY"]
title_cell(ws_sav, 1, 1, len(sav_cols), "SAVINGS GOALS TRACKER")
title_cell(ws_sav, 2, 1, len(sav_cols),
           "Monitor your savings goals and projected completion dates", subtitle=True)
for i, h in enumerate(sav_cols):
    col_header(ws_sav, 3, i+1, h)

goals = [
    ("Emergency Fund",    10000, 0, 500, "12/31/2026", "High"),
    ("Vacation",           3000, 0, 200, "07/01/2026", "Medium"),
    ("Home Down Payment", 50000, 0, 800, "01/01/2029", "High"),
    ("New Car",           15000, 0, 300, "06/01/2027", "Medium"),
    ("Education",          8000, 0, 250, "09/01/2027", "Medium"),
    ("Retirement Boost",  20000, 0, 400, "01/01/2030", "Low"),
]
priority_bg = {"High": TERRA_LIGHT, "Medium": SAGE_PALE, "Low": CREAM}

for j, (gname, gtgt, gcur, gmth, gdate, gpri) in enumerate(goals):
    r = 4 + j
    bg = priority_bg.get(gpri, CREAM)
    lbl(ws_sav, r, 1, gname, bold=True, bg=bg)
    inp(ws_sav, r, 2, gtgt, FMT_CURR, bg)
    inp(ws_sav, r, 3, gcur, FMT_CURR, bg)
    inp(ws_sav, r, 4, gmth, FMT_CURR, bg)
    cd = ws_sav.cell(row=r, column=5, value=gdate)
    cd.font = fn(9, False, INPUT_BLUE)
    cd.fill = fl(bg)
    cd.alignment = al("right")
    cd.border = thin_border()
    cd.number_format = FMT_DATE
    frm(ws_sav, r, 6, f"=IFERROR(C{r}/B{r},0)", FMT_PCT, bg=bg)
    # Progress bar
    pb = ws_sav.cell(row=r, column=7,
        value=f'=IFERROR(REPT(CHAR(9608),ROUND(F{r}*10,0))&REPT(CHAR(9617),10-ROUND(F{r}*10,0)),"          ")')
    pb.font = fn(9, False, SAGE_DARK)
    pb.fill = fl(bg)
    pb.alignment = al("left")
    pb.border = thin_border()
    # Months to goal (in notes column for space — we repurpose col 8 as priority)
    lbl(ws_sav, r, 8, gpri, bold=True, bg=bg)

goal_end = 4 + len(goals) - 1
tot_r_sav = goal_end + 1
# Write totals row manually (avoid merge collision on col 2)
_sav_tot_items = [
    (1, "TOTALS",                                                          "@",     True),
    (2, f"=SUM(B4:B{goal_end})",                                          FMT_CURR, False),
    (3, f"=SUM(C4:C{goal_end})",                                          FMT_CURR, False),
    (4, f"=SUM(D4:D{goal_end})",                                          FMT_CURR, False),
    (5, "",                                                                "@",     False),
    (6, f"=IFERROR(SUM(C4:C{goal_end})/SUM(B4:B{goal_end}),0)",          FMT_PCT,  False),
    (7, "",                                                                "@",     False),
    (8, "",                                                                "@",     False),
]
ws_sav.row_dimensions[tot_r_sav].height = 20
for _col, _val, _fmt, _bold in _sav_tot_items:
    _c = ws_sav.cell(row=tot_r_sav, column=_col, value=_val)
    _c.font = fn(9, True, CHARCOAL)
    _c.fill = fl(SAGE_LIGHT)
    _c.alignment = al("right" if _col > 1 else "left")
    _c.border = bottom_med()
    _c.number_format = _fmt

freeze(ws_sav, "A3")
set_widths(ws_sav, {"A":22,"B":14,"C":14,"D":16,"E":12,"F":12,"G":16,"H":10})


# ═══════════════════════════════════════════════════════════════════════════
# 6. ANNUAL OVERVIEW
# ═══════════════════════════════════════════════════════════════════════════
ws_ann = wb.create_sheet("Annual Overview")
no_gridlines(ws_ann)
ws_ann.sheet_properties.tabColor = SAGE_DARK

total_ann_cols = 1 + 12 + 3  # A + 12 months + Annual Total + Budget + Variance
title_cell(ws_ann, 1, 1, total_ann_cols, "ANNUAL OVERVIEW · 2026")
title_cell(ws_ann, 2, 1, total_ann_cols,
           "Year-at-a-glance: pull from all 12 monthly sheets", subtitle=True)

ann_headers = ["LINE ITEM"] + MONTHS + ["ANNUAL TOTAL","ANNUAL BUDGET","VARIANCE"]
for i, h in enumerate(ann_headers):
    col_header(ws_ann, 3, i+1, h)

# Row map: label → how to pull from monthly sheet
# We need the actual D (Actual) column row number per category per month.
# ti_row and gt_row are consistent across all months (same structure).
# subtotals dict maps cat_name -> row number (consistent since all months identical).

ann_row_defs = [
    ("INCOME",                  "ti_row",   None),
    ("HOUSING",                 "sub",      "HOUSING"),
    ("TRANSPORT",               "sub",      "TRANSPORT"),
    ("FOOD & DINING",           "sub",      "FOOD & DINING"),
    ("HEALTH & WELLNESS",       "sub",      "HEALTH & WELLNESS"),
    ("LIFESTYLE",               "sub",      "LIFESTYLE"),
    ("PERSONAL",                "sub",      "PERSONAL"),
    ("SAVINGS & INVESTMENTS",   "sub",      "SAVINGS & INVESTMENTS"),
    ("DEBT PAYMENTS",           "sub",      "DEBT PAYMENTS"),
    ("GRAND TOTAL EXPENSES",    "gt_row",   None),
    ("NET CASH FLOW",           "ncf_row",  None),
]

income_ann_row = None
expenses_ann_row = None

for i, (label, key, cat) in enumerate(ann_row_defs):
    r = 4 + i
    is_bold = label in ("INCOME","GRAND TOTAL EXPENSES","NET CASH FLOW")
    bg_label = TERRA if label in ("GRAND TOTAL EXPENSES",) else \
               SAGE_DARK if label == "NET CASH FLOW" else \
               SAGE_LIGHT if label == "INCOME" else \
               SAGE_PALE if i % 2 else CREAM
    txt_color = WHITE if label in ("NET CASH FLOW",) else CHARCOAL

    lbl(ws_ann, r, 1, label, bold=is_bold,
        bg=bg_label, color=txt_color)

    for m_i, m_short in enumerate(MONTHS):
        col = 2 + m_i
        meta = month_meta[m_short]
        if key == "ti_row":
            sheet_row = meta["ti_row"]
        elif key == "gt_row":
            sheet_row = meta["gt_row"]
        elif key == "ncf_row":
            sheet_row = meta["ncf_row"]
        else:  # sub
            sheet_row = meta["subtotals"][cat]

        fc = ws_ann.cell(row=r, column=col,
                         value=f"=IFERROR({m_short}!D{sheet_row},0)")
        fc.font = fn(9, is_bold, LINK_GREEN)
        fc.fill = fl(bg_label)
        fc.alignment = al("right")
        fc.border = thin_border()
        fc.number_format = FMT_CURR

    # Annual Total
    col_n = 14
    fc_n = ws_ann.cell(row=r, column=col_n,
                       value=f"=SUM(B{r}:M{r})")
    fc_n.font = fn(9, True, CHARCOAL)
    fc_n.fill = fl(bg_label)
    fc_n.alignment = al("right")
    fc_n.border = thin_border()
    fc_n.number_format = FMT_CURR

    # Annual Budget (input)
    inp(ws_ann, r, 15, 0, FMT_CURR, bg_label)

    # Variance
    fc_p = ws_ann.cell(row=r, column=16,
                       value=f"=IFERROR(N{r}-O{r},0)")
    fc_p.font = fn(9, is_bold, CHARCOAL)
    fc_p.fill = fl(bg_label)
    fc_p.alignment = al("right")
    fc_p.border = thin_border()
    fc_p.number_format = FMT_CURR

    if label == "INCOME":            income_ann_row   = r
    if label == "GRAND TOTAL EXPENSES": expenses_ann_row = r

# KEY METRICS section
km_start = 4 + len(ann_row_defs) + 2
section_hdr(ws_ann, km_start, 1, 5, "KEY METRICS")
km_items = [
    ("Best Month (lowest spend)",
     f'=IFERROR(INDEX(B3:M3,MATCH(MIN(B{expenses_ann_row}:M{expenses_ann_row}),B{expenses_ann_row}:M{expenses_ann_row},0)),"N/A")'),
    ("Worst Month (highest spend)",
     f'=IFERROR(INDEX(B3:M3,MATCH(MAX(B{expenses_ann_row}:M{expenses_ann_row}),B{expenses_ann_row}:M{expenses_ann_row},0)),"N/A")'),
    ("Annual Savings Rate",
     f"=IFERROR((N{income_ann_row}-N{expenses_ann_row})/N{income_ann_row},0)"),
    ("Average Monthly Spend",
     f"=IFERROR(N{expenses_ann_row}/12,0)"),
]
for k, (km_lbl, km_frm) in enumerate(km_items):
    r = km_start + 1 + k
    bg = SAGE_PALE if k % 2 else CREAM
    ws_ann.merge_cells(start_row=r, start_column=1, end_row=r, end_column=3)
    lbl(ws_ann, r, 1, km_lbl, bold=True, bg=bg)
    for col in [2, 3]:
        ws_ann.cell(row=r, column=col).fill = fl(bg)
        ws_ann.cell(row=r, column=col).border = thin_border()
    fmt = FMT_PCT if "Rate" in km_lbl else ("@" if "Month" in km_lbl else FMT_CURR)
    ws_ann.merge_cells(start_row=r, start_column=4, end_row=r, end_column=5)
    frm(ws_ann, r, 4, km_frm, fmt, LINK_GREEN, bg)
    ws_ann.cell(row=r, column=5).fill = fl(bg)
    ws_ann.cell(row=r, column=5).border = thin_border()

freeze(ws_ann, "B3")
set_widths(ws_ann, {
    "A": 26,
    **{get_column_letter(2+i): 10 for i in range(12)},
    "N": 14, "O": 14, "P": 14
})


# ═══════════════════════════════════════════════════════════════════════════
# FINAL: reorder sheets to canonical order
# ═══════════════════════════════════════════════════════════════════════════
ordered = (["Dashboard"] + MONTHS +
           ["Debt Tracker","Bill Calendar","Savings Goals","Annual Overview"])
# wb already has them in order; just confirm
assert wb.sheetnames == ordered, f"Sheet order mismatch: {wb.sheetnames}"

OUT = "ultimate_budget_tracker.xlsx"
wb.save(OUT)
print(f"Saved: {OUT}")
print(f"Sheets ({len(wb.sheetnames)}): {', '.join(wb.sheetnames)}")
