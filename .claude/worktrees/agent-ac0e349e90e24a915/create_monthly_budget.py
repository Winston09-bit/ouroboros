import openpyxl
from openpyxl import Workbook
from openpyxl.styles import (PatternFill, Font, Alignment, Border, Side,
                              GradientFill)
from openpyxl.utils import get_column_letter
from openpyxl.chart import BarChart, Reference, DoughnutChart
from openpyxl.chart.series import DataPoint

# ── PALETTE ──────────────────────────────────────────────────────────────────
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

def fill(hex_color):
    return PatternFill("solid", fgColor=hex_color)

def font(bold=False, color=WHITE, size=11, italic=False):
    return Font(bold=bold, color=color, size=size, italic=italic, name="Calibri")

def align(h="center", v="center", wrap=False):
    return Alignment(horizontal=h, vertical=v, wrap_text=wrap)

def thin_border():
    s = Side(style='thin', color='DDDDDD')
    return Border(left=s, right=s, top=s, bottom=s)

# ── DATA ─────────────────────────────────────────────────────────────────────
MO_NAMES = ["January","February","March","April","May","June",
            "July","August","September","October","November","December"]
MO_SHORT = ["Jan","Feb","Mar","Apr","May","Jun",
            "Jul","Aug","Sep","Oct","Nov","Dec"]

INCOME_SOURCES = ["Primary Salary","Freelance","Rental Income",
                  "Dividends","Side Business","Other Income"]

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
CAT_KEYS = list(EXPENSE_CATS.keys())

SAVINGS_GOALS = [
    ("Emergency Fund",     50000),
    ("Vacation — Japan",   25000),
    ("Home Down Payment", 250000),
    ("New Car",            80000),
    ("Retirement Boost",  100000),
    ("Education Fund",     40000),
    ("Wedding Fund",       60000),
    ("Home Renovation",    35000),
]

# ── HELPERS ───────────────────────────────────────────────────────────────────
def set_col_widths(ws, widths):
    """widths = list of (col_index_1based, width)"""
    for idx, w in widths:
        ws.column_dimensions[get_column_letter(idx)].width = w

def write_cell(ws, row, col, value=None, fill_c=None, font_obj=None,
               align_obj=None, num_fmt=None, border=None):
    c = ws.cell(row=row, column=col, value=value)
    if fill_c:   c.fill       = fill_c
    if font_obj: c.font       = font_obj
    if align_obj:c.alignment  = align_obj
    if num_fmt:  c.number_format = num_fmt
    if border:   c.border     = border
    return c

def merge_row(ws, row, col_start, col_end, value=None, fill_c=None,
              font_obj=None, height=None, align_obj=None):
    ws.merge_cells(start_row=row, start_column=col_start,
                   end_row=row, end_column=col_end)
    c = ws.cell(row=row, column=col_start, value=value)
    if fill_c:   c.fill      = fill_c
    if font_obj: c.font      = font_obj
    if align_obj:c.alignment = align_obj
    if height:   ws.row_dimensions[row].height = height
    return c

# ── BUILD MONTHLY SHEET ───────────────────────────────────────────────────────
def build_monthly(ws, mo_idx):
    """Returns rows dict for this month sheet."""
    mo_name = MO_NAMES[mo_idx]
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "A4"

    # Column widths: A=22, B=22, C=13, D=13, E=13, F=10, G=18
    set_col_widths(ws, [(1,22),(2,22),(3,13),(4,13),(5,13),(6,10),(7,18)])

    r = {}  # row tracker

    # Row 1 – title
    merge_row(ws, 1, 1, 7,
              value=f"{mo_name.upper()} · MONTHLY BUDGET",
              fill_c=fill(ROSE_DARK),
              font_obj=Font(bold=True, color=WHITE, size=15, name="Calibri"),
              align_obj=align("center","center"),
              height=44)

    # Row 2 – subtitle
    merge_row(ws, 2, 1, 7,
              value="Track your income, expenses, and savings for the month",
              fill_c=fill(GOLD),
              font_obj=Font(italic=True, color=WHITE, size=10, name="Calibri"),
              align_obj=align("center","center"),
              height=18)

    # Row 3 – headers
    headers = ["CATEGORY","SUBCATEGORY","BUDGETED","ACTUAL","DIFFERENCE","% USED","NOTES"]
    ws.row_dimensions[3].height = 20
    for ci, h in enumerate(headers, 1):
        write_cell(ws, 3, ci, h,
                   fill_c=fill(ROSE_DARK),
                   font_obj=Font(bold=True, color=WHITE, size=10, name="Calibri"),
                   align_obj=align("center","center"))

    cur = 4  # current row

    # ── INCOME section ─────────────────────────────────────────────────────
    r['income_section'] = cur
    merge_row(ws, cur, 1, 7, "INCOME",
              fill_c=fill(SAGE_DARK),
              font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
              align_obj=align("left","center"))
    ws.row_dimensions[cur].height = 18
    cur += 1

    r['income_start'] = cur
    for i, src in enumerate(INCOME_SOURCES):
        row_fill = fill(ROSE_PALE) if i % 2 == 0 else fill(WHITE)
        write_cell(ws, cur, 1, "INCOME", fill_c=row_fill,
                   font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("left","center"))
        write_cell(ws, cur, 2, src, fill_c=row_fill,
                   font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("left","center"))
        # C = Budgeted (input)
        write_cell(ws, cur, 3, 0, fill_c=row_fill,
                   font_obj=Font(color=INPUT_BLUE, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        # D = Actual (input)
        write_cell(ws, cur, 4, 0, fill_c=row_fill,
                   font_obj=Font(color=INPUT_BLUE, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        # E = Difference = C - D
        c_ref = f"C{cur}"
        d_ref = f"D{cur}"
        write_cell(ws, cur, 5, f"={c_ref}-{d_ref}", fill_c=row_fill,
                   font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        # F = % Used = IFERROR(D/C, 0)
        write_cell(ws, cur, 6, f"=IFERROR({d_ref}/{c_ref},0)", fill_c=row_fill,
                   font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=PCT_FMT)
        # G = Notes
        write_cell(ws, cur, 7, "", fill_c=row_fill,
                   font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("left","center"))
        cur += 1

    r['income_end'] = cur - 1

    # Total Income row
    r['income_total'] = cur
    merge_row(ws, cur, 1, 2, "TOTAL INCOME",
              fill_c=fill(SAGE_LIGHT),
              font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
              align_obj=align("left","center"))
    ws.row_dimensions[cur].height = 18
    # C = SUM budgeted
    write_cell(ws, cur, 3,
               f"=SUM(C{r['income_start']}:C{r['income_end']})",
               fill_c=fill(SAGE_LIGHT),
               font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    # D = SUM actual
    write_cell(ws, cur, 4,
               f"=SUM(D{r['income_start']}:D{r['income_end']})",
               fill_c=fill(SAGE_LIGHT),
               font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    # E = C-D
    write_cell(ws, cur, 5,
               f"=C{cur}-D{cur}",
               fill_c=fill(SAGE_LIGHT),
               font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    # F = IFERROR(D/C,0)
    write_cell(ws, cur, 6,
               f"=IFERROR(D{cur}/C{cur},0)",
               fill_c=fill(SAGE_LIGHT),
               font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
               align_obj=align("right","center"), num_fmt=PCT_FMT)
    write_cell(ws, cur, 7, "", fill_c=fill(SAGE_LIGHT))
    cur += 1

    # ── EXPENSE CATEGORIES ──────────────────────────────────────────────────
    for cat_key, subs in EXPENSE_CATS.items():
        cat_safe = cat_key  # used as dict key

        r[f'{cat_safe}_section'] = cur
        merge_row(ws, cur, 1, 7, cat_key,
                  fill_c=fill(ROSE_MID),
                  font_obj=Font(bold=True, color=WHITE, size=10, name="Calibri"),
                  align_obj=align("left","center"))
        ws.row_dimensions[cur].height = 18
        cur += 1

        r[f'{cat_safe}_start'] = cur
        for i, sub in enumerate(subs):
            row_fill = fill(ROSE_PALE) if i % 2 == 0 else fill(WHITE)
            write_cell(ws, cur, 1, cat_key, fill_c=row_fill,
                       font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                       align_obj=align("left","center"))
            write_cell(ws, cur, 2, sub, fill_c=row_fill,
                       font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                       align_obj=align("left","center"))
            write_cell(ws, cur, 3, 0, fill_c=row_fill,
                       font_obj=Font(color=INPUT_BLUE, size=10, name="Calibri"),
                       align_obj=align("right","center"), num_fmt=MONEY_FMT)
            write_cell(ws, cur, 4, 0, fill_c=row_fill,
                       font_obj=Font(color=INPUT_BLUE, size=10, name="Calibri"),
                       align_obj=align("right","center"), num_fmt=MONEY_FMT)
            write_cell(ws, cur, 5, f"=C{cur}-D{cur}", fill_c=row_fill,
                       font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                       align_obj=align("right","center"), num_fmt=MONEY_FMT)
            write_cell(ws, cur, 6, f"=IFERROR(D{cur}/C{cur},0)", fill_c=row_fill,
                       font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                       align_obj=align("right","center"), num_fmt=PCT_FMT)
            write_cell(ws, cur, 7, "", fill_c=row_fill,
                       font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                       align_obj=align("left","center"))
            cur += 1

        r[f'{cat_safe}_end'] = cur - 1

        # Subtotal row
        r[f'{cat_safe}_total'] = cur
        merge_row(ws, cur, 1, 2, f"{cat_key} TOTAL",
                  fill_c=fill(ROSE_LIGHT),
                  font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
                  align_obj=align("left","center"))
        ws.row_dimensions[cur].height = 18
        write_cell(ws, cur, 3,
                   f"=SUM(C{r[f'{cat_safe}_start']}:C{r[f'{cat_safe}_end']})",
                   fill_c=fill(ROSE_LIGHT),
                   font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        write_cell(ws, cur, 4,
                   f"=SUM(D{r[f'{cat_safe}_start']}:D{r[f'{cat_safe}_end']})",
                   fill_c=fill(ROSE_LIGHT),
                   font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        write_cell(ws, cur, 5,
                   f"=C{cur}-D{cur}",
                   fill_c=fill(ROSE_LIGHT),
                   font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        write_cell(ws, cur, 6,
                   f"=IFERROR(D{cur}/C{cur},0)",
                   fill_c=fill(ROSE_LIGHT),
                   font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=PCT_FMT)
        write_cell(ws, cur, 7, "", fill_c=fill(ROSE_LIGHT))
        cur += 1

    # ── GRAND TOTAL EXPENSES ────────────────────────────────────────────────
    r['grand_total'] = cur
    cat_total_rows = [r[f'{k}_total'] for k in CAT_KEYS]
    gt_sum = "+".join([f"D{row}" for row in cat_total_rows])
    merge_row(ws, cur, 1, 2, "GRAND TOTAL EXPENSES",
              fill_c=fill(ROSE_DARK),
              font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
              align_obj=align("left","center"))
    ws.row_dimensions[cur].height = 22
    write_cell(ws, cur, 3, f"=SUM({','.join([f'C{row}' for row in cat_total_rows])})",
               fill_c=fill(ROSE_DARK),
               font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    write_cell(ws, cur, 4, f"={gt_sum}",
               fill_c=fill(ROSE_DARK),
               font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    write_cell(ws, cur, 5, f"=C{cur}-D{cur}",
               fill_c=fill(ROSE_DARK),
               font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    write_cell(ws, cur, 6, f"=IFERROR(D{cur}/C{cur},0)",
               fill_c=fill(ROSE_DARK),
               font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
               align_obj=align("right","center"), num_fmt=PCT_FMT)
    write_cell(ws, cur, 7, "", fill_c=fill(ROSE_DARK))
    cur += 1

    # ── NET CASH FLOW ────────────────────────────────────────────────────────
    r['net_flow'] = cur
    merge_row(ws, cur, 1, 2, "NET CASH FLOW",
              fill_c=fill(SAGE_DARK),
              font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
              align_obj=align("left","center"))
    ws.row_dimensions[cur].height = 22
    it = r['income_total']
    gt = r['grand_total']
    write_cell(ws, cur, 3, f"=C{it}-C{gt}",
               fill_c=fill(SAGE_DARK),
               font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    write_cell(ws, cur, 4, f"=D{it}-D{gt}",
               fill_c=fill(SAGE_DARK),
               font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    write_cell(ws, cur, 5, f"=C{cur}-D{cur}",
               fill_c=fill(SAGE_DARK),
               font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
               align_obj=align("right","center"), num_fmt=MONEY_FMT)
    write_cell(ws, cur, 6, "",
               fill_c=fill(SAGE_DARK))
    write_cell(ws, cur, 7, "", fill_c=fill(SAGE_DARK))

    return r


# ── BUILD DASHBOARD ───────────────────────────────────────────────────────────
def build_dashboard(ws, rows):
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "A3"

    # Col widths: A=18, B-N=13 each
    set_col_widths(ws, [(1,18)] + [(i,13) for i in range(2,15)])

    # Row 1 – title
    merge_row(ws, 1, 1, 14,
              "DASHBOARD · THE ULTIMATE MONTHLY BUDGET · 2026",
              fill_c=fill(ROSE_DARK),
              font_obj=Font(bold=True, color=WHITE, size=16, name="Calibri"),
              align_obj=align("center","center"),
              height=46)

    # Row 2 – subtitle
    merge_row(ws, 2, 1, 14,
              "Your complete financial picture at a glance",
              fill_c=fill(GOLD),
              font_obj=Font(italic=True, color=WHITE, size=11, name="Calibri"),
              align_obj=align("center","center"),
              height=20)

    # Row 3 – section header
    merge_row(ws, 3, 1, 14, "BUDGET OVERVIEW",
              fill_c=fill(ROSE_MID),
              font_obj=Font(bold=True, color=WHITE, size=11, name="Calibri"),
              align_obj=align("center","center"),
              height=20)

    # KPI rows 4-9
    kpi_fills = [ROSE_PALE, WHITE, ROSE_PALE, WHITE, ROSE_PALE, WHITE]

    # Build income_total, grand_total, sav_total cross-sheet refs
    inc_refs = "+".join([f"{mo}!D{rows[mo]['income_total']}" for mo in MO_SHORT])
    exp_refs = "+".join([f"{mo}!D{rows[mo]['grand_total']}" for mo in MO_SHORT])
    sav_refs = "+".join([f"{mo}!D{rows[mo]['SAVINGS & INVESTMENTS_total']}" for mo in MO_SHORT])

    net_refs = [f"{mo}!D{rows[mo]['net_flow']}" for mo in MO_SHORT]

    kpis = [
        ("Annual Income",   f"={inc_refs}", MONEY_FMT),
        ("Annual Expenses", f"={exp_refs}", MONEY_FMT),
        ("Annual Savings",  f"={sav_refs}", MONEY_FMT),
        ("Savings Rate",    f"=IFERROR(B6/B4,0)", PCT_FMT),
        ("Best Month",
         '=INDEX({"Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"},MATCH(MAX('
         + ",".join(net_refs) + '),{' + ",".join(net_refs) + '},0))', "@"),
        ("Worst Month",
         '=INDEX({"Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"},MATCH(MIN('
         + ",".join(net_refs) + '),{' + ",".join(net_refs) + '},0))', "@"),
    ]

    for i, (label, formula, fmt) in enumerate(kpis):
        row = 4 + i
        ws.row_dimensions[row].height = 17
        f_c = fill(kpi_fills[i])
        write_cell(ws, row, 1, label, fill_c=f_c,
                   font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("left","center"))
        write_cell(ws, row, 2, formula, fill_c=f_c,
                   font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=fmt)
        for col in range(3,15):
            write_cell(ws, row, col, fill_c=f_c)

    # ── HELPER DATA (rows 10-20) ─────────────────────────────────────────────
    # Row 10: headers
    ws.cell(row=10, column=1, value="Metric")
    for i, mo in enumerate(MO_SHORT):
        ws.cell(row=10, column=2+i, value=mo)

    # Row 11: Monthly Income
    ws.cell(row=11, column=1, value="Monthly Income")
    for i, mo in enumerate(MO_SHORT):
        ws.cell(row=11, column=2+i,
                value=f"={mo}!D{rows[mo]['income_total']}")
        ws.cell(row=11, column=2+i).number_format = MONEY_FMT

    # Row 12: Cumulative Income
    ws.cell(row=12, column=1, value="Cumulative Income")
    for i in range(12):
        col = 2 + i
        if i == 0:
            ws.cell(row=12, column=col, value=f"=B11")
        else:
            prev = get_column_letter(col-1)
            cur_let = get_column_letter(col)
            ws.cell(row=12, column=col, value=f"={prev}12+{cur_let}11")
        ws.cell(row=12, column=col).number_format = MONEY_FMT

    # Row 13: Monthly Expenses
    ws.cell(row=13, column=1, value="Monthly Expenses")
    for i, mo in enumerate(MO_SHORT):
        ws.cell(row=13, column=2+i,
                value=f"={mo}!D{rows[mo]['grand_total']}")
        ws.cell(row=13, column=2+i).number_format = MONEY_FMT

    # Row 14: Monthly Savings
    ws.cell(row=14, column=1, value="Monthly Savings")
    for i, mo in enumerate(MO_SHORT):
        ws.cell(row=14, column=2+i,
                value=f"={mo}!D{rows[mo]['SAVINGS & INVESTMENTS_total']}")
        ws.cell(row=14, column=2+i).number_format = MONEY_FMT

    # Row 15: Cumulative Savings
    ws.cell(row=15, column=1, value="Cumul Savings")
    for i in range(12):
        col = 2 + i
        if i == 0:
            ws.cell(row=15, column=col, value="=B14")
        else:
            prev = get_column_letter(col-1)
            cur_let = get_column_letter(col)
            ws.cell(row=15, column=col, value=f"={prev}15+{cur_let}14")
        ws.cell(row=15, column=col).number_format = MONEY_FMT

    # Row 16: Net Flow
    ws.cell(row=16, column=1, value="Net Flow")
    for i, mo in enumerate(MO_SHORT):
        ws.cell(row=16, column=2+i,
                value=f"={mo}!D{rows[mo]['net_flow']}")
        ws.cell(row=16, column=2+i).number_format = MONEY_FMT

    # Row 17: expense cat names, Row 18: annual amounts per cat
    ws.cell(row=17, column=1, value="Category")
    ws.cell(row=18, column=1, value="Annual Amount")
    for j, cat in enumerate(CAT_KEYS):
        col = 2 + j
        ws.cell(row=17, column=col, value=cat)
        # Sum across all months
        cat_sum = "+".join([f"{mo}!D{rows[mo][f'{cat}_total']}" for mo in MO_SHORT])
        ws.cell(row=18, column=col, value=f"={cat_sum}")
        ws.cell(row=18, column=col).number_format = MONEY_FMT

    # Row 19: savings goal names, Row 20: amounts
    ws.cell(row=19, column=1, value="Goal")
    ws.cell(row=20, column=1, value="Target")
    for j, (goal, amount) in enumerate(SAVINGS_GOALS):
        col = 2 + j
        ws.cell(row=19, column=col, value=goal)
        ws.cell(row=20, column=col, value=amount)
        ws.cell(row=20, column=col).number_format = MONEY_FMT

    # ── CHART HELPER TABLES at cols P(16)+  ─────────────────────────────────
    # Doughnut chart helper (rows 3-10, cols P=16, Q=17)
    ws.cell(row=3, column=16, value="Category")
    ws.cell(row=3, column=17, value="Amount")
    for j, cat in enumerate(CAT_KEYS):
        row = 4 + j
        ws.cell(row=row, column=16, value=cat)
        cat_sum = "+".join([f"{mo}!D{rows[mo][f'{cat}_total']}" for mo in MO_SHORT])
        ws.cell(row=row, column=17, value=f"={cat_sum}")
        ws.cell(row=row, column=17).number_format = MONEY_FMT

    # Savings goals helper (rows 12-20, cols P=16, Q=17)
    ws.cell(row=12, column=16, value="Goal")
    ws.cell(row=12, column=17, value="Target")
    for j, (goal, amount) in enumerate(SAVINGS_GOALS):
        row = 13 + j
        ws.cell(row=row, column=16, value=goal)
        ws.cell(row=row, column=17, value=amount)
        ws.cell(row=row, column=17).number_format = MONEY_FMT

    # ── CHARTS ───────────────────────────────────────────────────────────────
    # Chart 1: Bar chart – Monthly Income (anchor A22)
    chart1 = BarChart()
    chart1.type = "col"
    chart1.title = "Annual Income by Month"
    chart1.y_axis.title = "Amount"
    chart1.x_axis.title = "Month"
    chart1.width = 14
    chart1.height = 10
    data1 = Reference(ws, min_col=2, max_col=13, min_row=11, max_row=11)
    cats1 = Reference(ws, min_col=2, max_col=13, min_row=10, max_row=10)
    chart1.add_data(data1)
    chart1.set_categories(cats1)
    if chart1.series:
        chart1.series[0].graphicalProperties.solidFill = CHART_ROSE
    ws.add_chart(chart1, "A22")

    # Chart 2: Doughnut chart – Expense by Category (anchor H22)
    chart2 = DoughnutChart()
    chart2.title = "Annual Expense by Category"
    chart2.width = 14
    chart2.height = 10
    data2 = Reference(ws, min_col=17, max_col=17, min_row=4, max_row=11)
    cats2 = Reference(ws, min_col=16, max_col=16, min_row=4, max_row=11)
    chart2.add_data(data2)
    chart2.set_categories(cats2)
    # Color slices
    for k, hex_c in enumerate(CHART_DONUT):
        pt = DataPoint(idx=k)
        pt.graphicalProperties.solidFill = hex_c
        chart2.series[0].dPt.append(pt)
    ws.add_chart(chart2, "H22")

    # Chart 3: Horizontal bar – Savings by Goal (anchor A42)
    chart3 = BarChart()
    chart3.type = "bar"
    chart3.title = "Annual Savings by Goal"
    chart3.y_axis.title = "Goal"
    chart3.x_axis.title = "Amount"
    chart3.width = 14
    chart3.height = 10
    data3 = Reference(ws, min_col=17, max_col=17, min_row=13, max_row=20)
    cats3 = Reference(ws, min_col=16, max_col=16, min_row=13, max_row=20)
    chart3.add_data(data3)
    chart3.set_categories(cats3)
    ws.add_chart(chart3, "A42")

    # Chart 4: Bar chart – Monthly Savings (anchor H42)
    chart4 = BarChart()
    chart4.type = "col"
    chart4.title = "Annual Savings by Month"
    chart4.y_axis.title = "Amount"
    chart4.x_axis.title = "Month"
    chart4.width = 14
    chart4.height = 10
    data4 = Reference(ws, min_col=2, max_col=13, min_row=14, max_row=14)
    cats4 = Reference(ws, min_col=2, max_col=13, min_row=10, max_row=10)
    chart4.add_data(data4)
    chart4.set_categories(cats4)
    if chart4.series:
        chart4.series[0].graphicalProperties.solidFill = CHART_GREEN
    ws.add_chart(chart4, "H42")


# ── BUILD ANNUAL BUDGET ───────────────────────────────────────────────────────
def build_annual_budget(ws, rows):
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "B4"

    # Col widths: A=26, B-M=10 each, N=13, O=12, P=12
    set_col_widths(ws, [(1,26)] + [(i,10) for i in range(2,14)] + [(14,13),(15,12),(16,12)])

    # Row 1
    merge_row(ws, 1, 1, 16, "ANNUAL BUDGET · 2026",
              fill_c=fill(ROSE_DARK),
              font_obj=Font(bold=True, color=WHITE, size=15, name="Calibri"),
              align_obj=align("center","center"), height=36)

    # Row 2
    merge_row(ws, 2, 1, 16, "Full year comparison across all months",
              fill_c=fill(GOLD),
              font_obj=Font(italic=True, color=WHITE, size=10, name="Calibri"),
              align_obj=align("center","center"), height=18)

    # Row 3 headers
    headers = ["LINE ITEM"] + MO_SHORT + ["ANNUAL TOTAL","ANNUAL BUDGET","VARIANCE"]
    ws.row_dimensions[3].height = 20
    for ci, h in enumerate(headers, 1):
        write_cell(ws, 3, ci, h,
                   fill_c=fill(ROSE_DARK),
                   font_obj=Font(bold=True, color=WHITE, size=10, name="Calibri"),
                   align_obj=align("center","center"))

    cur = 4

    def write_annual_row(label, d_refs, row_fill_hex, bold=False):
        nonlocal cur
        f_c = fill(row_fill_hex)
        fn = Font(bold=bold, color=CHARCOAL if row_fill_hex != ROSE_DARK else WHITE,
                  size=10, name="Calibri")
        write_cell(ws, cur, 1, label, fill_c=f_c, font_obj=fn,
                   align_obj=align("left","center"))
        for mi, ref in enumerate(d_refs):
            c = ws.cell(row=cur, column=2+mi, value=f"={ref}")
            c.fill = f_c
            c.font = fn
            c.alignment = align("right","center")
            c.number_format = MONEY_FMT
        # Col N = SUM(B:M)
        row_let = cur
        b_let = get_column_letter(2)
        m_let = get_column_letter(13)
        c_n = write_cell(ws, cur, 14, f"=SUM({b_let}{row_let}:{m_let}{row_let})",
                         fill_c=f_c, font_obj=fn,
                         align_obj=align("right","center"), num_fmt=MONEY_FMT)
        # Col O = 0 (input)
        c_o = write_cell(ws, cur, 15, 0, fill_c=f_c, font_obj=fn,
                         align_obj=align("right","center"), num_fmt=MONEY_FMT)
        # Col P = N-O
        write_cell(ws, cur, 16, f"=N{row_let}-O{row_let}",
                   fill_c=f_c, font_obj=fn,
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        cur += 1

    # INCOME row
    inc_refs = [f"{mo}!D{rows[mo]['income_total']}" for mo in MO_SHORT]
    write_annual_row("INCOME", inc_refs, SAGE_LIGHT, bold=True)

    # Expense category rows
    cat_fills = [ROSE_PALE, WHITE] * 4
    for ci, cat in enumerate(CAT_KEYS):
        cat_refs = [f"{mo}!D{rows[mo][f'{cat}_total']}" for mo in MO_SHORT]
        write_annual_row(cat, cat_refs, cat_fills[ci % 2])

    # Grand total expenses
    gt_refs = [f"{mo}!D{rows[mo]['grand_total']}" for mo in MO_SHORT]
    write_annual_row("GRAND TOTAL EXPENSES", gt_refs, ROSE_DARK, bold=True)

    # Net cash flow
    nf_refs = [f"{mo}!D{rows[mo]['net_flow']}" for mo in MO_SHORT]
    write_annual_row("NET CASH FLOW", nf_refs, SAGE_DARK, bold=True)


# ── BUILD SAVINGS TRACKER ─────────────────────────────────────────────────────
def build_savings_tracker(ws):
    ws.sheet_view.showGridLines = False
    ws.freeze_panes = "A4"

    set_col_widths(ws, [(1,26),(2,14),(3,14),(4,14),(5,12),(6,12),(7,18),(8,12)])

    # Row 1
    merge_row(ws, 1, 1, 8, "SAVINGS TRACKER · 2026",
              fill_c=fill(SAGE_DARK),
              font_obj=Font(bold=True, color=WHITE, size=15, name="Calibri"),
              align_obj=align("center","center"), height=36)

    # Row 2
    merge_row(ws, 2, 1, 8, "Monitor your progress towards financial goals",
              fill_c=fill(GOLD),
              font_obj=Font(italic=True, color=WHITE, size=10, name="Calibri"),
              align_obj=align("center","center"), height=18)

    # Row 3 headers
    headers = ["SAVINGS GOAL","TARGET","SAVED SO FAR","MONTHLY CONTRIB",
               "TARGET DATE","% COMPLETE","PROGRESS","PRIORITY"]
    ws.row_dimensions[3].height = 20
    for ci, h in enumerate(headers, 1):
        write_cell(ws, 3, ci, h,
                   fill_c=fill(ROSE_DARK),
                   font_obj=Font(bold=True, color=WHITE, size=10, name="Calibri"),
                   align_obj=align("center","center"))

    priorities = ["High","High","High","Medium","High","Medium","Low","Medium"]
    priority_colors = {"High": "C0392B", "Medium": "E67E22", "Low": "27AE60"}
    row_fills = [ROSE_PALE, SAGE_PALE] * 4

    for i, ((goal, target), priority) in enumerate(zip(SAVINGS_GOALS, priorities)):
        row = 4 + i
        rf = fill(row_fills[i])

        write_cell(ws, row, 1, goal, fill_c=rf,
                   font_obj=Font(bold=True, color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("left","center"))
        write_cell(ws, row, 2, target, fill_c=rf,
                   font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        write_cell(ws, row, 3, 0, fill_c=rf,
                   font_obj=Font(color=INPUT_BLUE, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        write_cell(ws, row, 4, 0, fill_c=rf,
                   font_obj=Font(color=INPUT_BLUE, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
        write_cell(ws, row, 5, "2026-12-31", fill_c=rf,
                   font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("center","center"))
        # F = IFERROR(C/B,0)
        write_cell(ws, row, 6, f"=IFERROR(C{row}/B{row},0)", fill_c=rf,
                   font_obj=Font(color=CHARCOAL, size=10, name="Calibri"),
                   align_obj=align("center","center"), num_fmt=PCT_FMT)
        # G = progress bar
        write_cell(ws, row, 7,
                   f'=REPT(CHAR(9608),ROUND(F{row}*10,0))&REPT(CHAR(9617),10-ROUND(F{row}*10,0))',
                   fill_c=rf,
                   font_obj=Font(color=SAGE_DARK, size=10, name="Calibri"),
                   align_obj=align("left","center"))
        # H = priority
        pc = priority_colors.get(priority, CHARCOAL)
        write_cell(ws, row, 8, priority, fill_c=rf,
                   font_obj=Font(bold=True, color=pc, size=10, name="Calibri"),
                   align_obj=align("center","center"))

    # Totals row
    tot_row = 4 + len(SAVINGS_GOALS)
    ws.row_dimensions[tot_row].height = 20
    merge_row(ws, tot_row, 1, 1, "TOTALS",
              fill_c=fill(ROSE_DARK),
              font_obj=Font(bold=True, color=WHITE, size=10, name="Calibri"),
              align_obj=align("left","center"))
    for col in [2,3,4]:
        col_let = get_column_letter(col)
        write_cell(ws, tot_row, col,
                   f"=SUM({col_let}4:{col_let}{tot_row-1})",
                   fill_c=fill(ROSE_DARK),
                   font_obj=Font(bold=True, color=WHITE, size=10, name="Calibri"),
                   align_obj=align("right","center"), num_fmt=MONEY_FMT)
    for col in range(5,9):
        write_cell(ws, tot_row, col, "", fill_c=fill(ROSE_DARK))


# ── MAIN ──────────────────────────────────────────────────────────────────────
def main():
    wb = Workbook()

    # Dashboard is the active sheet
    dash = wb.active
    dash.title = "Dashboard"

    # Create monthly sheets
    monthly_ws = {}
    for mo in MO_SHORT:
        ws = wb.create_sheet(mo)
        monthly_ws[mo] = ws

    # Create Annual Budget and Savings Tracker
    annual_ws = wb.create_sheet("Annual Budget")
    savings_ws = wb.create_sheet("Savings Tracker")

    # Build monthly sheets and collect row indices
    rows = {}
    for i, mo in enumerate(MO_SHORT):
        rows[mo] = build_monthly(monthly_ws[mo], i)

    # Build other sheets
    build_dashboard(dash, rows)
    build_annual_budget(annual_ws, rows)
    build_savings_tracker(savings_ws)

    out = "/home/user/ouroboros/ultimate_monthly_budget.xlsx"
    wb.save(out)
    print(f"Saved: {out}")


if __name__ == "__main__":
    main()
