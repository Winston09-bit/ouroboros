"""
Personal Finance Command Center — openpyxl generator
Output: personal_finance_tracker.xlsx
"""

from openpyxl import Workbook
from openpyxl.styles import (
    Font, PatternFill, Alignment, Border, Side, numbers
)
from openpyxl.utils import get_column_letter
from openpyxl.worksheet.datavalidation import DataValidation

wb = Workbook()

# ─── PALETTE ────────────────────────────────────────────────────────────────
C_GREEN       = "4A6741"
C_LIGHT_GREEN = "E8F5E3"
C_TERRA       = "C97B5A"
C_CREAM       = "FAF6F0"
C_WHITE       = "FFFFFF"
C_DARK        = "2C2C2C"
C_RED         = "FF4444"
C_YELLOW      = "FFD700"
C_PAID_GREEN  = "90EE90"

# text colours (Font RGB tuples)
BLUE  = "0000FF"
BLACK = "000000"
GREEN_TEXT = "008000"
WHITE_TEXT = "FFFFFF"

# ─── STYLE HELPERS ──────────────────────────────────────────────────────────
def fill(hex_color):
    return PatternFill("solid", fgColor=hex_color)

def font(bold=False, size=10, color=BLACK, name="Arial"):
    return Font(name=name, bold=bold, size=size, color=color)

def center():
    return Alignment(horizontal="center", vertical="center", wrap_text=True)

def left():
    return Alignment(horizontal="left", vertical="center", wrap_text=True)

def right():
    return Alignment(horizontal="right", vertical="center")

THIN  = Side(style="thin")
THICK = Side(style="medium")

def thin_border():
    return Border(left=THIN, right=THIN, top=THIN, bottom=THIN)

def thick_border():
    return Border(left=THICK, right=THICK, top=THICK, bottom=THICK)

def bottom_thick():
    return Border(left=THIN, right=THIN, top=THIN, bottom=THICK)

FMT_CURRENCY = '$#,##0.00'
FMT_PCT      = '0.0%'
FMT_DASH     = '_($* #,##0.00_);_($* (#,##0.00);_($* "-"??_);_(@_)'

def style_header(ws, cell_ref, text, font_size=14, bg=C_GREEN):
    cell = ws[cell_ref]
    cell.value = text
    cell.font  = font(bold=True, size=font_size, color=WHITE_TEXT)
    cell.fill  = fill(bg)
    cell.alignment = center()

def style_section_header(ws, row, col_start, col_end, text, bg=C_GREEN):
    ws.merge_cells(start_row=row, start_column=col_start,
                   end_row=row, end_column=col_end)
    cell = ws.cell(row=row, column=col_start, value=text)
    cell.font      = font(bold=True, size=11, color=WHITE_TEXT)
    cell.fill      = fill(bg)
    cell.alignment = center()
    cell.border    = thin_border()

def style_col_headers(ws, row, headers, col_start=1, bg=C_GREEN):
    for i, h in enumerate(headers):
        c = ws.cell(row=row, column=col_start + i, value=h)
        c.font      = font(bold=True, size=10, color=WHITE_TEXT)
        c.fill      = fill(bg)
        c.alignment = center()
        c.border    = thin_border()

def input_cell(ws, row, col, value=None, fmt=FMT_CURRENCY):
    c = ws.cell(row=row, column=col, value=value)
    c.font        = font(color=BLUE)
    c.alignment   = right()
    c.border      = thin_border()
    c.number_format = fmt
    return c

def formula_cell(ws, row, col, formula, fmt=FMT_CURRENCY, color=BLACK):
    c = ws.cell(row=row, column=col, value=formula)
    c.font          = font(color=color)
    c.alignment     = right()
    c.border        = thin_border()
    c.number_format = fmt
    return c

def label_cell(ws, row, col, text, bold=False, bg=None):
    c = ws.cell(row=row, column=col, value=text)
    c.font      = font(bold=bold, color=BLACK)
    c.alignment = left()
    c.border    = thin_border()
    if bg:
        c.fill = fill(bg)
    return c

def total_row_style(ws, row, cols, bg=C_LIGHT_GREEN):
    for col in cols:
        c = ws.cell(row=row, column=col)
        c.font   = font(bold=True, color=BLACK)
        c.fill   = fill(bg)
        c.border = bottom_thick()

def freeze(ws, cell="A3"):
    ws.freeze_panes = cell

def set_col_widths(ws, widths):
    for col_letter, w in widths.items():
        ws.column_dimensions[col_letter].width = w

MONTHS = ["Jan","Feb","Mar","Apr","May","Jun",
          "Jul","Aug","Sep","Oct","Nov","Dec"]
MONTH_NAMES = ["January","February","March","April","May","June",
               "July","August","September","October","November","December"]

# ═══════════════════════════════════════════════════════════════════════════
# 1. DASHBOARD
# ═══════════════════════════════════════════════════════════════════════════
ws_dash = wb.active
ws_dash.title = "Dashboard"

ws_dash.merge_cells("A1:L1")
style_header(ws_dash, "A1", "💰 Personal Finance Command Center", font_size=20, bg=C_GREEN)
ws_dash.row_dimensions[1].height = 40

# --- BUDGET SUMMARY BOX A3:F10 ---
style_section_header(ws_dash, 3, 1, 6, "📊 BUDGET SUMMARY", bg=C_GREEN)
summary_labels = [
    ("Annual Income", True),
    ("Monthly Income", False),
    ("Total Monthly Expenses", False),
    ("Left to Spend", False),
    ("Annual Savings Rate %", False),
    ("YTD Savings", False),
]
for i, (lbl, is_input) in enumerate(summary_labels):
    r = 4 + i
    label_cell(ws_dash, r, 1, lbl, bold=True)
    ws_dash.merge_cells(start_row=r, start_column=1, end_row=r, end_column=3)
    if is_input:
        input_cell(ws_dash, r, 4)
    else:
        if lbl == "Monthly Income":
            formula_cell(ws_dash, r, 4, "=IFERROR(D4/12,0)")
        elif lbl == "Total Monthly Expenses":
            # pull from Jan sheet grand total (row 50 is our grand total row — set later)
            formula_cell(ws_dash, r, 4, "=IFERROR(Jan!G50,0)", color=GREEN_TEXT)
        elif lbl == "Left to Spend":
            formula_cell(ws_dash, r, 4, "=IFERROR(D5-D6,0)")
        elif lbl == "Annual Savings Rate %":
            formula_cell(ws_dash, r, 4, "=IFERROR(D8/D4,0)", fmt=FMT_PCT)
        elif lbl == "YTD Savings":
            formula_cell(ws_dash, r, 4, "=IFERROR(Jan!G48,0)", color=GREEN_TEXT)
    ws_dash.merge_cells(start_row=r, start_column=4, end_row=r, end_column=6)

# Left to Spend conditional note (placed outside merged range, col 7)
note_cell = ws_dash.cell(row=7, column=7, value="← red if negative")
note_cell.font = font(size=9, color="999999")

# --- SAVINGS SNAPSHOT H3:L10 ---
style_section_header(ws_dash, 3, 8, 12, "💾 SAVINGS SNAPSHOT", bg=C_TERRA)
snap_labels = [
    ("Emergency Fund Goal", True),
    ("Current Emergency Fund", True),
    ("% Funded", False),
    ("Monthly Savings Target", True),
    ("YTD Savings", False),
]
for i, (lbl, is_input) in enumerate(snap_labels):
    r = 4 + i
    label_cell(ws_dash, r, 8, lbl, bold=True)
    ws_dash.merge_cells(start_row=r, start_column=8, end_row=r, end_column=10)
    if is_input:
        input_cell(ws_dash, r, 11)
    else:
        if lbl == "% Funded":
            formula_cell(ws_dash, r, 11, "=IFERROR(H5/H4,0)", fmt=FMT_PCT)
        elif lbl == "YTD Savings":
            formula_cell(ws_dash, r, 11, "=IFERROR(Jan!G48,0)", color=GREEN_TEXT)
    ws_dash.merge_cells(start_row=r, start_column=11, end_row=r, end_column=12)

# --- TOP SPENDING CATEGORIES A12:F20 ---
style_section_header(ws_dash, 12, 1, 6, "🏆 TOP SPENDING CATEGORIES (Current Month)", bg=C_TERRA)
style_col_headers(ws_dash, 13, ["Category","Budgeted","Actual","Difference","% Used","Notes"], bg=C_GREEN)
top_cats = [
    ("🏠 Housing",   "=IFERROR(Jan!D18,0)", "=IFERROR(Jan!E18,0)"),
    ("🚗 Transport", "=IFERROR(Jan!D24,0)", "=IFERROR(Jan!E24,0)"),
    ("🛒 Food",      "=IFERROR(Jan!D30,0)", "=IFERROR(Jan!E30,0)"),
    ("🏥 Health",    "=IFERROR(Jan!D36,0)", "=IFERROR(Jan!E36,0)"),
    ("🎬 Lifestyle", "=IFERROR(Jan!D42,0)", "=IFERROR(Jan!E42,0)"),
]
for i, (cat, budg, act) in enumerate(top_cats):
    r = 14 + i
    bg = C_CREAM if i % 2 else C_WHITE
    label_cell(ws_dash, r, 1, cat, bg=bg)
    ws_dash.merge_cells(start_row=r, start_column=1, end_row=r, end_column=2)
    formula_cell(ws_dash, r, 3, budg, color=GREEN_TEXT)
    formula_cell(ws_dash, r, 4, act,  color=GREEN_TEXT)
    formula_cell(ws_dash, r, 5, f"=IFERROR(C{r}-D{r},0)")
    formula_cell(ws_dash, r, 6, f"=IFERROR(D{r}/C{r},0)", fmt=FMT_PCT)

# --- BILLS DUE H12:L20 ---
style_section_header(ws_dash, 12, 8, 12, "📆 BILLS DUE THIS MONTH", bg=C_GREEN)
style_col_headers(ws_dash, 13, ["Bill","Amount","Due","Auto-Pay?","Status"], col_start=8, bg=C_TERRA)
# Pull first 5 bills from Bill Calendar
for i in range(5):
    r = 14 + i
    bg = C_CREAM if i % 2 else C_WHITE
    br = i + 3  # Bill Calendar data starts row 3
    label_cell(ws_dash, r, 8,  f"=IFERROR('Bill Calendar'!A{br},\"\")", bg=bg)
    formula_cell(ws_dash, r, 9,  f"=IFERROR('Bill Calendar'!C{br},0)", color=GREEN_TEXT)
    formula_cell(ws_dash, r, 10, f"=IFERROR('Bill Calendar'!D{br},\"\")", fmt="MM/DD", color=GREEN_TEXT)
    label_cell(ws_dash, r, 11, f"=IFERROR('Bill Calendar'!F{br},\"\")", bg=bg)
    label_cell(ws_dash, r, 12, f"=IFERROR('Bill Calendar'!G{br},\"\")", bg=bg)

# --- CHART PLACEHOLDER ---
ws_dash.merge_cells("A22:L35")
cell = ws_dash["A22"]
cell.value = "[ Insert Chart Here: Monthly Income vs Expenses ]"
cell.font      = font(bold=True, size=12, color=C_GREEN)
cell.alignment = center()
cell.fill      = fill(C_LIGHT_GREEN)
cell.border    = thick_border()
ws_dash.row_dimensions[22].height = 18

freeze(ws_dash, "A3")
set_col_widths(ws_dash, {
    "A":25,"B":15,"C":15,"D":15,"E":12,"F":15,
    "G":3,
    "H":25,"I":15,"J":12,"K":12,"L":15
})


# ═══════════════════════════════════════════════════════════════════════════
# 2. MONTHLY BUDGET SHEETS  (Jan–Dec)
# ═══════════════════════════════════════════════════════════════════════════
EXPENSE_CATEGORIES = [
    ("🏠 HOUSING",    ["Rent/Mortgage","Utilities","Internet","Renter's Insurance"]),
    ("🚗 TRANSPORT",  ["Car Payment","Gas","Insurance","Parking","Public Transit"]),
    ("🛒 FOOD",       ["Groceries","Dining Out","Coffee Shops","Meal Delivery"]),
    ("🏥 HEALTH",     ["Health Insurance","Gym","Pharmacy","Doctor Visits"]),
    ("🎬 LIFESTYLE",  ["Subscriptions","Entertainment","Hobbies","Personal Care"]),
    ("👔 PERSONAL",   ["Clothing","Gifts","Education","Miscellaneous"]),
    ("💰 SAVINGS",    ["Emergency Fund","Retirement (401k/IRA)","Investments","Vacation Fund"]),
    ("💳 DEBT",       ["Credit Card 1","Credit Card 2","Student Loan","Other Loan"]),
]
HEADERS = ["Category","Subcategory","Budgeted","Actual","Difference","% Used","Notes"]

month_subtotal_rows = {}  # month_name -> {cat_name: row}

for m_idx, (m_short, m_long) in enumerate(zip(MONTHS, MONTH_NAMES)):
    ws = wb.create_sheet(title=m_short)

    # Title row
    ws.merge_cells("A1:H1")
    style_header(ws, "A1", f"📅 Monthly Budget — {m_long}", font_size=14, bg=C_TERRA)
    ws.row_dimensions[1].height = 30

    # Column headers row 2
    style_col_headers(ws, 2, HEADERS, bg=C_GREEN)

    # INCOME SECTION
    style_section_header(ws, 3, 1, 7, "💵 INCOME", bg=C_GREEN)
    income_items = ["Primary Income","Side Income","Other Income"]
    income_rows = []
    for i, item in enumerate(income_items):
        r = 4 + i
        income_rows.append(r)
        label_cell(ws, r, 1, "Income")
        label_cell(ws, r, 2, item)
        input_cell(ws, r, 3)   # Budgeted
        input_cell(ws, r, 4)   # Actual
        formula_cell(ws, r, 5, f"=IFERROR(C{r}-D{r},0)")
        formula_cell(ws, r, 6, f"=IFERROR(D{r}/C{r},0)", fmt=FMT_PCT)
        label_cell(ws, r, 7, "")
        bg = C_CREAM if i % 2 else C_WHITE
        for col in range(1, 8):
            ws.cell(row=r, column=col).fill = fill(bg)

    # Total Income row
    ti_row = 4 + len(income_items)  # row 7
    ws.merge_cells(start_row=ti_row, start_column=1, end_row=ti_row, end_column=2)
    label_cell(ws, ti_row, 1, "TOTAL INCOME", bold=True, bg=C_LIGHT_GREEN)
    formula_cell(ws, ti_row, 3, f"=SUM(C4:C{ti_row-1})", color=BLACK)
    formula_cell(ws, ti_row, 4, f"=SUM(D4:D{ti_row-1})", color=BLACK)
    formula_cell(ws, ti_row, 5, f"=IFERROR(C{ti_row}-D{ti_row},0)")
    formula_cell(ws, ti_row, 6, f"=IFERROR(D{ti_row}/C{ti_row},0)", fmt=FMT_PCT)
    total_row_style(ws, ti_row, range(1, 8), bg=C_LIGHT_GREEN)

    # EXPENSE SECTIONS
    current_row = ti_row + 1
    cat_subtotal_rows = {}
    subtotal_row_list = []

    for cat_name, sub_items in EXPENSE_CATEGORIES:
        # Category header
        style_section_header(ws, current_row, 1, 7, cat_name, bg=C_GREEN)
        current_row += 1

        sub_start = current_row
        for j, sub in enumerate(sub_items):
            label_cell(ws, current_row, 1, cat_name.split(" ",1)[1])
            label_cell(ws, current_row, 2, sub)
            input_cell(ws, current_row, 3)
            input_cell(ws, current_row, 4)
            formula_cell(ws, current_row, 5, f"=IFERROR(C{current_row}-D{current_row},0)")
            formula_cell(ws, current_row, 6, f"=IFERROR(D{current_row}/C{current_row},0)", fmt=FMT_PCT)
            label_cell(ws, current_row, 7, "")
            bg = C_CREAM if j % 2 else C_WHITE
            for col in range(1, 8):
                ws.cell(row=current_row, column=col).fill = fill(bg)
            current_row += 1

        sub_end = current_row - 1
        # Subtotal row
        ws.merge_cells(start_row=current_row, start_column=1, end_row=current_row, end_column=2)
        label_cell(ws, current_row, 1, f"Subtotal {cat_name}", bold=True, bg=C_CREAM)
        formula_cell(ws, current_row, 3, f"=SUM(C{sub_start}:C{sub_end})")
        formula_cell(ws, current_row, 4, f"=SUM(D{sub_start}:D{sub_end})")
        formula_cell(ws, current_row, 5, f"=IFERROR(C{current_row}-D{current_row},0)")
        formula_cell(ws, current_row, 6, f"=IFERROR(D{current_row}/C{current_row},0)", fmt=FMT_PCT)
        total_row_style(ws, current_row, range(1, 7), bg=C_CREAM)
        cat_subtotal_rows[cat_name] = current_row
        subtotal_row_list.append(current_row)
        current_row += 1

    month_subtotal_rows[m_short] = cat_subtotal_rows

    # GRAND TOTAL EXPENSES row
    grand_row = current_row
    ws.merge_cells(start_row=grand_row, start_column=1, end_row=grand_row, end_column=2)
    label_cell(ws, grand_row, 1, "GRAND TOTAL EXPENSES", bold=True, bg=C_TERRA)
    ws.cell(row=grand_row, column=1).font = font(bold=True, color=WHITE_TEXT)
    subtotal_refs_budg = "+".join([f"C{r}" for r in subtotal_row_list])
    subtotal_refs_act  = "+".join([f"D{r}" for r in subtotal_row_list])
    formula_cell(ws, grand_row, 3, f"={subtotal_refs_budg}")
    formula_cell(ws, grand_row, 4, f"={subtotal_refs_act}")
    formula_cell(ws, grand_row, 5, f"=IFERROR(C{grand_row}-D{grand_row},0)")
    formula_cell(ws, grand_row, 6, f"=IFERROR(D{grand_row}/C{grand_row},0)", fmt=FMT_PCT)
    total_row_style(ws, grand_row, range(1, 7), bg=C_TERRA)
    for col in range(1, 8):
        ws.cell(row=grand_row, column=col).fill = fill(C_TERRA)

    # NET CASH FLOW row
    net_row = grand_row + 1
    ws.merge_cells(start_row=net_row, start_column=1, end_row=net_row, end_column=2)
    label_cell(ws, net_row, 1, "NET CASH FLOW", bold=True)
    formula_cell(ws, net_row, 3, f"=IFERROR(C{ti_row}-C{grand_row},0)")
    formula_cell(ws, net_row, 4, f"=IFERROR(D{ti_row}-D{grand_row},0)")
    formula_cell(ws, net_row, 5, f"=IFERROR(C{net_row}-D{net_row},0)")
    total_row_style(ws, net_row, range(1, 7), bg=C_LIGHT_GREEN)

    # Store key row references for cross-sheet use
    ws["A1"].comment = None  # placeholder
    ws.sheet_properties.tabColor = C_GREEN

    freeze(ws, "A3")
    set_col_widths(ws, {"A":25,"B":22,"C":15,"D":15,"E":15,"F":10,"G":20})


# ═══════════════════════════════════════════════════════════════════════════
# 3. DEBT TRACKER
# ═══════════════════════════════════════════════════════════════════════════
ws_debt = wb.create_sheet("Debt Tracker")

ws_debt.merge_cells("A1:I1")
style_header(ws_debt, "A1", "💳 Debt Payoff Tracker", font_size=14, bg=C_TERRA)
ws_debt.row_dimensions[1].height = 30

debt_headers = ["Debt Name","Lender","Original Balance","Current Balance",
                "Interest Rate","Min Payment","Monthly Payment","Payoff Date","Status"]
style_col_headers(ws_debt, 2, debt_headers, bg=C_GREEN)

debts = [
    ("Credit Card 1",""),
    ("Credit Card 2",""),
    ("Student Loan",""),
    ("Car Loan",""),
    ("Personal Loan",""),
    ("Other",""),
]
for i, (name, lender) in enumerate(debts):
    r = 3 + i
    bg = C_CREAM if i % 2 else C_WHITE
    label_cell(ws_debt, r, 1, name, bg=bg)
    input_cell(ws_debt, r, 2, fmt="@")
    ws_debt.cell(row=r, column=2).number_format = "@"
    input_cell(ws_debt, r, 3)
    input_cell(ws_debt, r, 4)
    input_cell(ws_debt, r, 5, fmt="0.00%")
    input_cell(ws_debt, r, 6)
    input_cell(ws_debt, r, 7)
    input_cell(ws_debt, r, 8, fmt="MM/DD/YYYY")
    label_cell(ws_debt, r, 9, "Active", bg=bg)
    ws_debt.cell(row=r, column=9).fill = fill("FFFF99")
    for col in [1,2,3,4,5,6,7,8]:
        ws_debt.cell(row=r, column=col).fill = fill(bg)

# Avalanche ranking column header
ws_debt.cell(row=2, column=10).value = "Avalanche Rank"
ws_debt.cell(row=2, column=10).font = font(bold=True, color=WHITE_TEXT)
ws_debt.cell(row=2, column=10).fill = fill(C_GREEN)
ws_debt.cell(row=2, column=10).border = thin_border()
for i in range(6):
    r = 3 + i
    formula_cell(ws_debt, r, 10,
        f'=IFERROR(RANK(E{r},E3:E8,0),"N/A")', fmt="0", color=BLACK)

# SUMMARY BOX
sum_start = 11
style_section_header(ws_debt, sum_start, 1, 5, "📊 DEBT SUMMARY", bg=C_GREEN)
summary_items = [
    ("Total Debt",               "=IFERROR(SUM(D3:D8),0)"),
    ("Total Monthly Minimums",   "=IFERROR(SUM(F3:F8),0)"),
    ("Debt-to-Income Ratio",     "=IFERROR(SUM(F3:F8)/Dashboard!D5,0)"),
    ("Est. Total Interest",      '=IFERROR(SUMPRODUCT(D3:D8,E3:E8)/12*12,"N/A")'),
]
for i, (lbl, frm) in enumerate(summary_items):
    r = sum_start + 1 + i
    label_cell(ws_debt, r, 1, lbl, bold=True)
    ws_debt.merge_cells(start_row=r, start_column=1, end_row=r, end_column=3)
    fmt = FMT_PCT if "Ratio" in lbl else FMT_CURRENCY
    formula_cell(ws_debt, r, 4, frm, fmt=fmt, color=GREEN_TEXT)
    ws_debt.merge_cells(start_row=r, start_column=4, end_row=r, end_column=5)

freeze(ws_debt, "A3")
set_col_widths(ws_debt, {"A":20,"B":18,"C":16,"D":16,"E":14,"F":14,"G":14,"H":14,"I":12,"J":14})


# ═══════════════════════════════════════════════════════════════════════════
# 4. BILL CALENDAR
# ═══════════════════════════════════════════════════════════════════════════
ws_bill = wb.create_sheet("Bill Calendar")

ws_bill.merge_cells("A1:H1")
style_header(ws_bill, "A1", "📆 Bill Calendar", font_size=14, bg=C_GREEN)
ws_bill.row_dimensions[1].height = 30

bill_headers = ["Bill Name","Category","Amount","Due Date","Frequency","Auto-Pay?","Paid?","Notes"]
style_col_headers(ws_bill, 2, bill_headers, bg=C_TERRA)

sample_bills = [
    ("Rent/Mortgage",  "Housing",       1500, "2026-01-01", "Monthly",  "No",  "No"),
    ("Electric",       "Utilities",       120, "2026-01-15", "Monthly",  "Yes", "No"),
    ("Internet",       "Utilities",        60, "2026-01-22", "Monthly",  "Yes", "No"),
    ("Netflix",        "Subscriptions",    18, "2026-01-08", "Monthly",  "Yes", "No"),
    ("Phone",          "Subscriptions",    85, "2026-01-12", "Monthly",  "Yes", "No"),
    ("Car Insurance",  "Insurance",       150, "2026-01-20", "Monthly",  "No",  "No"),
    ("Gym",            "Other",            40, "2026-01-01", "Monthly",  "Yes", "No"),
    ("Spotify",        "Subscriptions",    11, "2026-01-05", "Monthly",  "Yes", "No"),
]
for i, bill in enumerate(sample_bills):
    r = 3 + i
    bg = C_CREAM if i % 2 else C_WHITE
    name, cat, amt, due, freq, auto, paid = bill
    label_cell(ws_bill, r, 1, name, bg=bg)
    label_cell(ws_bill, r, 2, cat,  bg=bg)
    input_cell(ws_bill, r, 3, amt)
    c_date = ws_bill.cell(row=r, column=4, value=due)
    c_date.font = font(color=BLUE)
    c_date.number_format = "MM/DD/YYYY"
    c_date.alignment = right()
    c_date.border = thin_border()
    label_cell(ws_bill, r, 5, freq,  bg=bg)
    label_cell(ws_bill, r, 6, auto,  bg=bg)
    label_cell(ws_bill, r, 7, paid,  bg=bg)
    label_cell(ws_bill, r, 8, "",    bg=bg)

# Add empty rows for additional bills
for i in range(8, 20):
    r = 3 + i
    bg = C_CREAM if i % 2 else C_WHITE
    for col in range(1, 9):
        c = ws_bill.cell(row=r, column=col, value="")
        c.fill   = fill(bg)
        c.border = thin_border()
        if col == 3:
            c.number_format = FMT_CURRENCY

# Monthly total
tot_row = 24
ws_bill.merge_cells(start_row=tot_row, start_column=1, end_row=tot_row, end_column=2)
label_cell(ws_bill, tot_row, 1, "MONTHLY TOTAL", bold=True, bg=C_LIGHT_GREEN)
formula_cell(ws_bill, tot_row, 3, "=SUM(C3:C23)")
total_row_style(ws_bill, tot_row, range(1, 9), bg=C_LIGHT_GREEN)

# Data validation
dv_cat  = DataValidation(type="list", formula1='"Housing,Utilities,Insurance,Subscriptions,Loans,Other"', allow_blank=True)
dv_freq = DataValidation(type="list", formula1='"Monthly,Weekly,Bi-Weekly,Quarterly,Annual"', allow_blank=True)
dv_yn   = DataValidation(type="list", formula1='"Yes,No"', allow_blank=True)
ws_bill.add_data_validation(dv_cat);  dv_cat.sqref  = "B3:B23"
ws_bill.add_data_validation(dv_freq); dv_freq.sqref = "E3:E23"
ws_bill.add_data_validation(dv_yn);   dv_yn.sqref   = "F3:G23"

freeze(ws_bill, "A3")
set_col_widths(ws_bill, {"A":22,"B":16,"C":14,"D":14,"E":14,"F":12,"G":10,"H":22})


# ═══════════════════════════════════════════════════════════════════════════
# 5. SAVINGS GOALS
# ═══════════════════════════════════════════════════════════════════════════
ws_sav = wb.create_sheet("Savings Goals")

ws_sav.merge_cells("A1:H1")
style_header(ws_sav, "A1", "🎯 Savings Goals Tracker", font_size=14, bg=C_GREEN)
ws_sav.row_dimensions[1].height = 30

sav_headers = ["Goal Name","Target Amount","Current Saved","Monthly Contribution",
               "Target Date","% Complete","Progress","Priority"]
style_col_headers(ws_sav, 2, sav_headers, bg=C_TERRA)

goals = [
    ("Emergency Fund",    10000, 0, 500, "2026-12-31", "High"),
    ("Vacation",           3000, 0, 200, "2026-07-01", "Medium"),
    ("Down Payment",      50000, 0, 800, "2029-01-01", "High"),
    ("New Car",           15000, 0, 300, "2027-06-01", "Medium"),
    ("Retirement Boost",  20000, 0, 400, "2030-01-01", "Low"),
    ("Other",              5000, 0, 100, "2027-01-01", "Low"),
]
for i, (name, target, current, monthly, tdate, priority) in enumerate(goals):
    r = 3 + i
    bg = C_CREAM if i % 2 else C_WHITE
    label_cell(ws_sav, r, 1, name, bg=bg)
    input_cell(ws_sav, r, 2, target)
    input_cell(ws_sav, r, 3, current)
    input_cell(ws_sav, r, 4, monthly)
    c_date = ws_sav.cell(row=r, column=5, value=tdate)
    c_date.font = font(color=BLUE)
    c_date.number_format = "MM/DD/YYYY"
    c_date.alignment = right()
    c_date.border = thin_border()
    formula_cell(ws_sav, r, 6, f"=IFERROR(C{r}/B{r},0)", fmt=FMT_PCT)
    # Progress bar
    pb = ws_sav.cell(row=r, column=7,
        value=f'=IFERROR(REPT("█",ROUND(C{r}/B{r}*20,0))&REPT("░",20-ROUND(C{r}/B{r}*20,0)),"░░░░░░░░░░░░░░░░░░░░")')
    pb.font      = font(color=C_GREEN)
    pb.alignment = left()
    pb.border    = thin_border()
    label_cell(ws_sav, r, 8, priority, bg=bg)
    for col in [1, 2, 3, 4, 5, 8]:
        ws_sav.cell(row=r, column=col).fill = fill(bg)

# TOTAL ROW
tot_row = 3 + len(goals)
ws_sav.merge_cells(start_row=tot_row, start_column=1, end_row=tot_row, end_column=1)
label_cell(ws_sav, tot_row, 1, "TOTALS", bold=True, bg=C_LIGHT_GREEN)
for col, frm in [(2, f"=SUM(B3:B{tot_row-1})"),
                  (3, f"=SUM(C3:C{tot_row-1})"),
                  (4, f"=SUM(D3:D{tot_row-1})")]:
    formula_cell(ws_sav, tot_row, col, frm)
total_row_style(ws_sav, tot_row, range(1, 9), bg=C_LIGHT_GREEN)

dv_priority = DataValidation(type="list", formula1='"High,Medium,Low"', allow_blank=True)
ws_sav.add_data_validation(dv_priority)
dv_priority.sqref = "H3:H8"

freeze(ws_sav, "A3")
set_col_widths(ws_sav, {"A":22,"B":16,"C":16,"D":18,"E":14,"F":12,"G":25,"H":12})


# ═══════════════════════════════════════════════════════════════════════════
# 6. ANNUAL OVERVIEW
# ═══════════════════════════════════════════════════════════════════════════
ws_ann = wb.create_sheet("Annual Overview")

ws_ann.merge_cells("A1:N1")
style_header(ws_ann, "A1", "📊 Annual Summary 2026", font_size=16, bg=C_GREEN)
ws_ann.row_dimensions[1].height = 35

# Column headers: Category + 12 months + Annual Total + Annual Budget + Variance
ann_headers = ["Category"] + MONTHS + ["Annual Total","Annual Budget","Variance"]
style_col_headers(ws_ann, 2, ann_headers, bg=C_TERRA)

# Row definitions: label, and which cell to pull from each monthly sheet
# We need to know row numbers from the monthly sheets.
# Key rows in monthly sheets (approximate based on structure):
# ti_row = 7 (total income), grand_row = 50+ depending on items
# We'll use a dynamic approach referencing named rows via our known structure:
# Income total row = 7, Grand total expenses row = let's calculate.
# Actually per our loop: ti_row=7, expense categories add up:
# Each cat: 1 header row + N subs + 1 subtotal = varies
# Housing: 4+1+1=6 rows from row 8 → subtotal at 13+1=14? Let me recalc.
# ti_row = 4+3=7
# First category starts at row 8:
#   HOUSING header: 8, subs 9-12, subtotal 13
#   TRANSPORT header: 14, subs 15-19, subtotal 20
#   FOOD header: 21, subs 22-25, subtotal 26
#   HEALTH header: 27, subs 28-31, subtotal 32
#   LIFESTYLE header: 33, subs 34-37, subtotal 38
#   PERSONAL header: 39, subs 40-43, subtotal 44
#   SAVINGS header: 45, subs 46-49, subtotal 50
#   DEBT header: 51, subs 52-55, subtotal 56
# grand_row = 57, net_row = 58

ANN_ROW_MAP = {
    "Income":            ("D", 7),   # col D = Actual, row 7
    "Total Expenses":    ("D", 57),
    "Housing":           ("D", 13),
    "Transport":         ("D", 20),
    "Food":              ("D", 26),
    "Health":            ("D", 32),
    "Lifestyle":         ("D", 38),
    "Personal":          ("D", 44),
    "Savings":           ("D", 50),
    "Debt Payments":     ("D", 56),
    "Net Cash Flow":     ("D", 58),
}

ann_row_labels = list(ANN_ROW_MAP.keys())
for i, label in enumerate(ann_row_labels):
    r = 3 + i
    bg = C_CREAM if i % 2 else C_WHITE
    label_cell(ws_ann, r, 1, label, bold=(label in ["Income","Total Expenses","Net Cash Flow"]), bg=bg)
    col_ref, sheet_row = ANN_ROW_MAP[label]
    for m_i, m_short in enumerate(MONTHS):
        col = 2 + m_i
        formula_cell(ws_ann, r, col,
            f"=IFERROR({m_short}!{col_ref}{sheet_row},0)", color=GREEN_TEXT)
    # Annual Total
    col_start_letter = get_column_letter(2)
    col_end_letter   = get_column_letter(13)
    formula_cell(ws_ann, r, 14, f"=SUM(B{r}:M{r})")
    input_cell(ws_ann, r, 15)        # Annual Budget
    formula_cell(ws_ann, r, 16, f"=IFERROR(N{r}-O{r},0)")

# Bottom summary
sum_r = 3 + len(ann_row_labels) + 1
style_section_header(ws_ann, sum_r, 1, 8, "📈 ANNUAL STATISTICS", bg=C_GREEN)

stats = [
    ("YTD Savings Rate",          f"=IFERROR(N{3+ann_row_labels.index('Savings')}/N{3+ann_row_labels.index('Income')},0)", FMT_PCT),
    ("Highest Spending Month",    f"=IFERROR(INDEX(A2:M2,MATCH(MAX(B{3+ann_row_labels.index('Total Expenses')}:M{3+ann_row_labels.index('Total Expenses')}),B{3+ann_row_labels.index('Total Expenses')}:M{3+ann_row_labels.index('Total Expenses')},0)+1,1),\"N/A\")", "@"),
    ("Lowest Spending Month",     f"=IFERROR(INDEX(A2:M2,MATCH(MIN(B{3+ann_row_labels.index('Total Expenses')}:M{3+ann_row_labels.index('Total Expenses')}),B{3+ann_row_labels.index('Total Expenses')}:M{3+ann_row_labels.index('Total Expenses')},0)+1,1),\"N/A\")", "@"),
    ("Total Annual Income",       f"=IFERROR(N{3+ann_row_labels.index('Income')},0)", FMT_CURRENCY),
    ("Total Annual Expenses",     f"=IFERROR(N{3+ann_row_labels.index('Total Expenses')},0)", FMT_CURRENCY),
    ("Annual Net Cash Flow",      f"=IFERROR(N{3+ann_row_labels.index('Net Cash Flow')},0)", FMT_CURRENCY),
]
for i, (lbl, frm, fmt) in enumerate(stats):
    r = sum_r + 1 + i
    label_cell(ws_ann, r, 1, lbl, bold=True)
    ws_ann.merge_cells(start_row=r, start_column=1, end_row=r, end_column=3)
    formula_cell(ws_ann, r, 4, frm, fmt=fmt, color=GREEN_TEXT)
    ws_ann.merge_cells(start_row=r, start_column=4, end_row=r, end_column=6)

freeze(ws_ann, "A3")
set_col_widths(ws_ann, {
    "A":22,**{get_column_letter(2+i): 10 for i in range(12)},
    "N":14,"O":14,"P":14
})


# ═══════════════════════════════════════════════════════════════════════════
# GLOBAL: Tab colours and sheet order check
# ═══════════════════════════════════════════════════════════════════════════
for m in MONTHS:
    wb[m].sheet_properties.tabColor = C_GREEN
ws_dash.sheet_properties.tabColor   = "2E7D32"
ws_debt.sheet_properties.tabColor   = C_TERRA
ws_bill.sheet_properties.tabColor   = C_TERRA
ws_sav.sheet_properties.tabColor    = C_GREEN
ws_ann.sheet_properties.tabColor    = "1B5E20"

# ═══════════════════════════════════════════════════════════════════════════
# SAVE
# ═══════════════════════════════════════════════════════════════════════════
OUT = "personal_finance_tracker.xlsx"
wb.save(OUT)
print(f"✅  Saved: {OUT}")
print(f"   Sheets ({len(wb.sheetnames)}): {', '.join(wb.sheetnames)}")
